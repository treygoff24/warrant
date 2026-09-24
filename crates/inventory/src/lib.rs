//! Deterministic path classification and completeness accounting.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use cargo_metadata::MetadataCommand;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use warrant_core::manifest::{ClassDeclaration, WarrantManifest};
use warrant_core::nouns::{
    Entrypoint, GeneratedAbsent, GeneratedBy, InventoryClass, InventoryDocument, InventoryEntry,
    InventorySummary, SnapshotKind, SnapshotManifest, Submodule, UnitAliasTable, UnreadPath,
    VendoredFrom,
};

/// A module selector assigns ownership without changing a path's class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSelector {
    pub id: String,
    pub files: Vec<String>,
}

/// An explicit class assignment. `replaces` names an integration default it changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassRule {
    pub id: String,
    pub class: InventoryClass,
    pub files: Vec<String>,
    pub replaces: Option<InventoryClass>,
}

/// Inputs not represented by `warrant.yaml` yet (module policy arrives in a later milestone).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildConfig {
    pub modules: Vec<ModuleSelector>,
    pub class_rules: Vec<ClassRule>,
}

/// A discovered compilation or package scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unit {
    pub root: String,
    pub configuration: String,
    #[serde(default)]
    pub by: String,
}

/// Why a snapshot refused a read, in the snapshot's own terms (`snapshot-changed`, or
/// the entry's `unread` reason), so a caller can retake the snapshot or report it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadError {
    pub code: String,
    pub reason: String,
}

/// Reads captured bytes for one snapshot path.
pub type Reader<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, ReadError>;

/// The captured snapshot inventory classifies: its identity, path listing and bytes.
/// Discovery reads configuration only through `read`, never from the live filesystem.
#[derive(Clone, Copy)]
pub struct CapturedSnapshot<'a> {
    pub manifest: &'a SnapshotManifest,
    pub entries: &'a [InventoryEntry],
    pub read: Reader<'a>,
    /// Captured paths Git does not track (untracked, non-ignored worktree files). Every
    /// entry of an index, commit or tree snapshot is tracked, so this is empty for them.
    pub untracked: &'a BTreeSet<String>,
}

/// The untracked, non-ignored paths a capture of `kind` includes: the worktree's
/// `git ls-files --others --exclude-standard`, the same listing the capture adds to the
/// index; nothing for an object snapshot, whose entries all come from Git objects.
pub fn untracked_paths(
    root: &Path,
    kind: &SnapshotKind,
) -> Result<BTreeSet<String>, InventoryError> {
    if *kind != SnapshotKind::Worktree {
        return Ok(BTreeSet::new());
    }
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1");
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(variable);
    }
    let output = command
        .output()
        .map_err(|error| io_error("git ls-files --others", error))?;
    if !output.status.success() {
        return Err(io_error(
            "git ls-files --others",
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            std::str::from_utf8(record)
                .map(str::to_owned)
                .map_err(|_| io_error("git ls-files --others", "non-UTF-8 path"))
        })
        .collect()
}

/// Result of classifying a snapshot tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltInventory {
    pub document: InventoryDocument,
    pub digest: String,
    pub units: Vec<Unit>,
}

/// A reproducible producer mismatch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeneratedIssueCode {
    GeneratedDrift,
    GeneratedAbsent,
}

/// One generated-path verification result that needs attention.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeneratedIssue {
    pub code: GeneratedIssueCode,
    pub path: String,
    pub producer: String,
}

#[derive(Debug)]
pub enum InventoryError {
    Io {
        path: String,
        reason: String,
    },
    /// The snapshot refused to supply a path's captured bytes.
    Read {
        path: String,
        code: String,
        reason: String,
    },
    InvalidGlob {
        pattern: String,
        reason: String,
    },
    ClassificationConflict {
        path: String,
        rules: Vec<String>,
    },
    MissingDefaultReplacement {
        path: String,
        default: InventoryClass,
        rule: String,
    },
    WrongDefaultReplacement {
        path: String,
        expected: InventoryClass,
        named: InventoryClass,
        rule: String,
    },
    OwnershipOverlap {
        path: String,
        modules: Vec<String>,
    },
    NestedRepository {
        path: String,
    },
    InvalidDeclaration {
        reason: String,
    },
    ProducerFailed {
        producer: String,
        status: Option<i32>,
    },
}

impl InventoryError {
    /// Spec 12.1: errors are distinguishable by code, so each variant has its own stable
    /// code; spec names are used where they exist (`nested-repository`, 5.6). A refused
    /// snapshot read keeps the snapshot's code, which capture's retry depends on.
    pub fn code(&self) -> &str {
        match self {
            Self::Io { .. } => "inventory-io",
            Self::Read { code, .. } => code,
            Self::InvalidGlob { .. } => "invalid-glob",
            Self::ClassificationConflict { .. } => "classification-conflict",
            Self::MissingDefaultReplacement { .. } => "missing-default-replacement",
            Self::WrongDefaultReplacement { .. } => "wrong-default-replacement",
            Self::OwnershipOverlap { .. } => "ownership-overlap",
            Self::NestedRepository { .. } => "nested-repository",
            Self::InvalidDeclaration { .. } => "invalid-declaration",
            Self::ProducerFailed { .. } => "producer-failed",
        }
    }
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, reason } => write!(formatter, "could not read `{path}`: {reason}"),
            Self::Read { path, code, reason } => write!(
                formatter,
                "could not read `{path}` from the snapshot: {code}: {reason}"
            ),
            Self::InvalidGlob { pattern, reason } => {
                write!(formatter, "invalid glob `{pattern}`: {reason}")
            }
            Self::ClassificationConflict { path, rules } => {
                write!(
                    formatter,
                    "conflicting class rules for `{path}`: {}",
                    rules.join(", ")
                )
            }
            Self::MissingDefaultReplacement {
                path,
                default,
                rule,
            } => write!(
                formatter,
                "class rule `{rule}` changes the {} default for `{path}` without naming it",
                default.as_str()
            ),
            Self::WrongDefaultReplacement {
                path,
                expected,
                named,
                rule,
            } => write!(
                formatter,
                "class rule `{rule}` names {} for `{path}`, expected {}",
                named.as_str(),
                expected.as_str()
            ),
            Self::OwnershipOverlap { path, modules } => {
                write!(
                    formatter,
                    "overlapping module selectors for `{path}`: {}",
                    modules.join(", ")
                )
            }
            Self::NestedRepository { path } => write!(
                formatter,
                "`{path}` is a nested repository and not a declared submodule"
            ),
            Self::InvalidDeclaration { reason } => formatter.write_str(reason),
            Self::ProducerFailed { producer, status } => {
                write!(
                    formatter,
                    "generated producer `{producer}` failed with status {status:?}"
                )
            }
        }
    }
}

impl std::error::Error for InventoryError {}

/// Enrich the snapshot's path listing with inventory classification and provenance.
pub fn build(
    root: &Path,
    snapshot: CapturedSnapshot<'_>,
    manifest: &WarrantManifest,
    config: &BuildConfig,
) -> Result<BuiltInventory, InventoryError> {
    let snapshot_entries = snapshot.entries;
    let read = snapshot.read;
    // Spec 5.6: the worktree snapshot records an untracked nested repository as the
    // gitlink Git would stage, with this reason. It is not a declared submodule.
    if let Some(path) = snapshot_entries
        .iter()
        .filter(|entry| entry.reason == UNDECLARED_NESTED_REPOSITORY)
        .map(|entry| &entry.path)
        .min()
    {
        return Err(InventoryError::NestedRepository { path: path.clone() });
    }
    let submodules: Vec<_> = snapshot_entries
        .iter()
        .filter(|entry| {
            entry.class == InventoryClass::Submodule && entry.reason != UNDECLARED_NESTED_REPOSITORY
        })
        .map(|entry| entry.path.clone())
        .collect();
    let mut paths: Vec<String> = snapshot_entries
        .iter()
        .filter(|entry| {
            !has_git_component(Path::new(&entry.path), Path::new(""))
                && !submodules.iter().any(|submodule| {
                    entry.path != *submodule && Path::new(&entry.path).starts_with(submodule)
                })
        })
        .map(|entry| entry.path.clone())
        .collect();
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(InventoryError::InvalidDeclaration {
            reason: "duplicate path in snapshot listing".into(),
        });
    }
    let looked_at: Vec<&str> = snapshot_entries
        .iter()
        .filter(|entry| {
            entry.class != InventoryClass::Ignored && paths.binary_search(&entry.path).is_ok()
        })
        .map(|entry| entry.path.as_str())
        .collect();
    check_nested_repositories(root, &looked_at, snapshot.untracked, &submodules)?;
    let modules = compile_modules(&config.modules)?;
    let mut rules = manifest_rules(&manifest.inventory.classes);
    rules.extend(config.class_rules.clone());
    let class_rules = compile_rules(&rules)?;
    let generated = compile_generated(manifest)?;
    let vendored = compile_vendored(manifest)?;
    let enabled = EnabledIntegrations::from_manifest(manifest);
    let mut entries = Vec::new();

    for snapshot_entry in snapshot_entries {
        let relative = &snapshot_entry.path;
        if paths.binary_search(relative).is_err() {
            continue;
        }
        if matches!(
            snapshot_entry.class,
            InventoryClass::Ignored | InventoryClass::Submodule | InventoryClass::Unread
        ) {
            let mut entry = snapshot_entry.clone();
            entry.blob = prefixed_git_oid(&entry.blob)?;
            entries.push(entry);
            continue;
        }
        let default = default_class(
            relative,
            enabled,
            snapshot.untracked.contains(relative.as_str()),
        );
        let explicit = matching_rules(relative, &class_rules);
        let generated_match = matching_generated(relative, &generated);
        let vendored_match = matching_vendored(relative, &vendored);
        let declaration_count =
            generated_match.len() + vendored_match.len() + usize::from(!explicit.is_empty());
        if declaration_count > 1 {
            let mut names: Vec<String> = explicit.iter().map(|rule| rule.rule.id.clone()).collect();
            names.extend(
                generated_match
                    .iter()
                    .map(|item| format!("manifest:inventory.generated[{}]", item.declaration)),
            );
            names.extend(
                vendored_match
                    .iter()
                    .map(|item| format!("manifest:inventory.vendored[{}]", item.declaration)),
            );
            names.sort();
            return Err(InventoryError::ClassificationConflict {
                path: relative.clone(),
                rules: names,
            });
        }

        let (class, by, reason, generated_by, vendored_from) =
            if let Some(item) = generated_match.first() {
                (
                    InventoryClass::Generated,
                    "manifest:inventory.generated".into(),
                    format!("matched generated glob {}", item.pattern),
                    Some(GeneratedBy {
                        producer: item.producer.clone(),
                        reproducible: item.reproducible,
                        inputs: item.inputs.clone(),
                    }),
                    None,
                )
            } else if let Some(item) = vendored_match.first() {
                (
                    InventoryClass::Vendored,
                    "manifest:inventory.vendored".into(),
                    format!("matched vendored glob {}", item.pattern),
                    None,
                    Some(VendoredFrom {
                        source: item.source.clone(),
                        version: item.version.clone(),
                        treatment: item.treatment.clone(),
                    }),
                )
            } else if let Some(rule) = choose_explicit(relative, default.0, &explicit)? {
                (
                    rule.rule.class,
                    format!("rule:{}", rule.rule.id),
                    format!("matched explicit class glob {}", rule.pattern),
                    None,
                    None,
                )
            } else {
                (default.0, default.1, default.2, None, None)
            };

        let owners = matching_modules(relative, &modules);
        if owners.len() > 1 {
            return Err(InventoryError::OwnershipOverlap {
                path: relative.clone(),
                modules: owners,
            });
        }
        let module = owners.into_iter().next();
        entries.push(InventoryEntry {
            path: relative.clone(),
            blob: prefixed_git_oid(&snapshot_entry.blob)?,
            class,
            language: language(relative, enabled),
            unit: None,
            module,
            by,
            reason,
            entrypoints: Vec::new(),
            unread: snapshot_entry.unread.clone(),
            generated_by,
            vendored_from,
        });
    }

    // An unread entry stays in the inventory with its reason; nothing it would have
    // declared (a unit, an alias table, an entrypoint) is inferred from bytes around it.
    let discovery_paths: Vec<String> = entries
        .iter()
        .filter(|entry| {
            entry.unread.is_none()
                && !matches!(
                    entry.class,
                    InventoryClass::Ignored
                        | InventoryClass::Submodule
                        | InventoryClass::BuildOutput
                        | InventoryClass::Vendored
                        | InventoryClass::Unread
                )
        })
        .map(|entry| entry.path.clone())
        .collect();
    // Cargo's inputs are every captured path, unread ones included: Cargo reads them
    // from a copy of the snapshot, and a refused read is refused there.
    let captured_paths: Vec<String> = snapshot_entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.class,
                InventoryClass::Ignored | InventoryClass::Submodule
            ) && paths.binary_search(&entry.path).is_ok()
        })
        .map(|entry| entry.path.clone())
        .collect();
    let mut units = discover_units_from_paths(read, &discovery_paths, &captured_paths)?;
    let package_entrypoints = discover_package_entrypoints(read, &discovery_paths)?;
    for entry in &mut entries {
        if matches!(
            entry.class,
            InventoryClass::Ignored | InventoryClass::Submodule | InventoryClass::Unread
        ) {
            continue;
        }
        entry.unit = unit_for(&entry.path, &units, source_language(&entry.path, enabled))
            .map(|unit| unit.root.clone());
        // `by` stays the classifying rule; the fallback unit records its own basis.
        if entry.class == InventoryClass::Source && entry.unit.is_none() {
            entry.unit = Some(".".into());
        }
        entry.entrypoints = entrypoints_for(&entry.path, manifest, &package_entrypoints);
    }

    if entries
        .iter()
        .any(|entry| entry.unit.as_deref() == Some("."))
        && !units.iter().any(|unit| unit.root == ".")
    {
        units.push(Unit {
            root: ".".into(),
            configuration: String::new(),
            by: "implicit-root-fallback".into(),
        });
        units.sort_by(|left, right| left.root.cmp(&right.root));
    }
    // Spec 5.3: an ignored file is absent from the snapshot even when a copy is on disk,
    // so generated presence is decided against captured paths, not the exclusion listing.
    let captured: Vec<&str> = snapshot_entries
        .iter()
        .filter(|entry| {
            entry.class != InventoryClass::Ignored && paths.binary_search(&entry.path).is_ok()
        })
        .map(|entry| entry.path.as_str())
        .collect();
    let generated_absent = add_absent_generated(&mut entries, &generated, &captured)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let unit_aliases = alias_tables(read, &discovery_paths, &units)?;
    let summary = summarize(
        &entries,
        &snapshot.manifest.kind,
        unit_aliases,
        generated_absent,
    );
    let document = InventoryDocument {
        schema_version: "warrant.inventory/1".into(),
        snapshot: Some(snapshot.manifest.clone()),
        total: entries.len() as u64,
        entries,
        summary,
        truncated: false,
        next_cursor: None,
    };
    let digest = inventory_digest(&document)?;
    Ok(BuiltInventory {
        document,
        digest,
        units,
    })
}

/// Fail when a path is selected by more than one module, independent of order.
pub fn lint_ownership(root: &Path, modules: &[ModuleSelector]) -> Result<(), InventoryError> {
    let paths = repository_paths(root)?;
    let compiled = compile_modules(modules)?;
    for path in paths {
        let owners = matching_modules(&path, &compiled);
        if owners.len() > 1 {
            return Err(InventoryError::OwnershipOverlap {
                path,
                modules: owners,
            });
        }
    }
    Ok(())
}

/// Discover JavaScript, TypeScript, and Cargo units without compiling source.
pub fn discover_units(root: &Path) -> Result<Vec<Unit>, InventoryError> {
    let paths = repository_paths(root)?;
    let read = |path: &str| {
        fs::read(root.join(path)).map_err(|error| ReadError {
            code: "io".into(),
            reason: error.to_string(),
        })
    };
    discover_units_from_paths(&read, &paths, &paths)
}

/// A captured path's Git mode (`100644`, `100755`, `120000`), as the snapshot recorded it.
pub type ModeOf<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Spec 5.3: re-run each reproducible producer over a copy of the captured snapshot and
/// compare what it writes with the captured bytes. The copy is the snapshot's readable,
/// non-ignored entries in their captured bytes and modes, never the disk tree: ignored
/// files and edits made after capture are not the source being verified. Paths a
/// reproducible declaration matches are left out of the copy for the producer to write.
pub fn verify_generated(
    snapshot: CapturedSnapshot<'_>,
    mode: ModeOf<'_>,
    manifest: &WarrantManifest,
) -> Result<Vec<GeneratedIssue>, InventoryError> {
    let declarations = compile_generated(manifest)?;
    let reproducible: Vec<&GeneratedPattern> = declarations
        .iter()
        .filter(|item| item.reproducible)
        .collect();
    if reproducible.is_empty() {
        return Ok(Vec::new());
    }
    let present: BTreeSet<&str> = snapshot
        .entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.class,
                InventoryClass::Ignored | InventoryClass::Submodule
            )
        })
        .map(|entry| entry.path.as_str())
        .collect();
    let temporary = tempfile::tempdir().map_err(|error| io_error("temporary directory", error))?;
    for entry in snapshot.entries {
        if !present.contains(entry.path.as_str())
            || entry.unread.is_some()
            || reproducible
                .iter()
                .any(|item| item.matcher.is_match(&entry.path))
        {
            continue;
        }
        let bytes = read_captured(snapshot.read, &entry.path)?;
        write_captured(
            temporary.path(),
            &entry.path,
            &bytes,
            mode(&entry.path).as_deref(),
        )?;
    }

    let producers: BTreeSet<&str> = reproducible
        .iter()
        .map(|item| item.producer.as_str())
        .collect();
    for producer in producers {
        let status = Command::new("sh")
            .arg("-c")
            .arg(producer)
            .current_dir(temporary.path())
            .status()
            .map_err(|error| io_error(producer, error))?;
        if !status.success() {
            return Err(InventoryError::ProducerFailed {
                producer: producer.into(),
                status: status.code(),
            });
        }
    }

    let generated_paths = repository_paths(temporary.path())?;
    let mut issues = Vec::new();
    for item in reproducible {
        let mut candidates: BTreeSet<String> = present
            .iter()
            .map(|path| (*path).to_owned())
            .chain(generated_paths.iter().cloned())
            .filter(|path| item.matcher.is_match(path.as_str()))
            .collect();
        if is_literal(&item.pattern) || candidates.is_empty() {
            candidates.insert(item.pattern.clone());
        }
        for path in candidates {
            let reproduced = temporary.path().join(&path);
            let code = if !present.contains(path.as_str()) {
                Some(GeneratedIssueCode::GeneratedAbsent)
            } else if !(reproduced.is_file() || reproduced.is_symlink())
                // A refused read (an oversize or external-symlink output) is an error,
                // never a silent pass.
                || digest_bytes(&read_captured(snapshot.read, &path)?) != blob_id(&reproduced)?
            {
                Some(GeneratedIssueCode::GeneratedDrift)
            } else {
                None
            };
            if let Some(code) = code {
                issues.push(GeneratedIssue {
                    code,
                    path,
                    producer: item.producer.clone(),
                });
            }
        }
    }
    issues.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.producer.cmp(&right.producer))
    });
    issues.dedup();
    Ok(issues)
}

/// Hash the deterministic inventory document, including completeness accounting and the
/// snapshot identity. The capture time is excluded: it names when, not what, was read.
pub fn inventory_digest(document: &InventoryDocument) -> Result<String, InventoryError> {
    let mut document = document.clone();
    if let Some(snapshot) = &mut document.snapshot {
        snapshot.taken_at.clear();
    }
    let bytes =
        serde_json::to_vec(&document).map_err(|error| InventoryError::InvalidDeclaration {
            reason: format!("inventory serialization failed: {error}"),
        })?;
    Ok(digest_bytes(&bytes))
}

#[derive(Clone, Copy)]
struct EnabledIntegrations {
    typescript: bool,
    rust: bool,
}

impl EnabledIntegrations {
    fn from_manifest(manifest: &WarrantManifest) -> Self {
        let enabled = |names: &[&str]| {
            manifest
                .integrations
                .iter()
                .any(|(name, config)| config.enabled && names.contains(&name.as_str()))
        };
        Self {
            typescript: enabled(&["lang-ts", "typescript", "ts"]),
            rust: enabled(&["lang-rust", "rust"]),
        }
    }
}

struct CompiledRule {
    rule: ClassRule,
    matcher: GlobSet,
    pattern: String,
}
struct CompiledModule {
    id: String,
    matcher: GlobSet,
}
struct GeneratedPattern {
    declaration: usize,
    matcher: GlobSet,
    pattern: String,
    producer: String,
    inputs: Option<Vec<String>>,
    reproducible: bool,
}
struct VendoredPattern {
    declaration: usize,
    matcher: GlobSet,
    pattern: String,
    source: String,
    version: String,
    treatment: String,
}

/// Spec 5.6: a nested repository that is not a declared submodule gives its files two
/// identities. Only directories the snapshot looks into are checked: every ancestor of
/// a captured non-ignored entry, and every directory Git's untracked, non-ignored
/// listing reports instead of descending (that is how Git lists a nested repository).
/// A repository under an ignored directory with no captured entries is not looked at.
fn check_nested_repositories(
    root: &Path,
    captured: &[&str],
    untracked: &BTreeSet<String>,
    submodules: &[String],
) -> Result<(), InventoryError> {
    let mut directories = BTreeSet::new();
    for path in captured {
        let mut ancestor = Path::new(path).parent();
        while let Some(directory) = ancestor.filter(|directory| *directory != Path::new("")) {
            if !directories.insert(directory.to_string_lossy().into_owned()) {
                break;
            }
            ancestor = directory.parent();
        }
    }
    directories.extend(
        untracked
            .iter()
            .filter_map(|path| path.strip_suffix('/'))
            .map(str::to_owned),
    );
    for directory in directories {
        let within_submodule = submodules
            .iter()
            .any(|submodule| Path::new(&directory).starts_with(submodule));
        if !within_submodule && fs::symlink_metadata(root.join(&directory).join(".git")).is_ok() {
            return Err(InventoryError::NestedRepository { path: directory });
        }
    }
    Ok(())
}

fn repository_paths(root: &Path) -> Result<Vec<String>, InventoryError> {
    let mut paths = Vec::new();
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .ignore(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false);
    for result in walker.build() {
        let entry = result.map_err(|error| io_error(root, error))?;
        if entry.path() == root || has_git_component(entry.path(), root) {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() || file_type.is_symlink() {
            paths.push(relative_path(root, entry.path())?);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn has_git_component(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root).ok().is_some_and(|relative| {
        relative
            .components()
            .any(|part| part == Component::Normal(".git".as_ref()))
    })
}

fn relative_path(root: &Path, path: &Path) -> Result<String, InventoryError> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| InventoryError::InvalidDeclaration {
            reason: error.to_string(),
        })
}

fn compile_glob(pattern: &str) -> Result<GlobSet, InventoryError> {
    let mut builder = GlobSetBuilder::new();
    builder.add(
        Glob::new(pattern).map_err(|error| InventoryError::InvalidGlob {
            pattern: pattern.into(),
            reason: error.to_string(),
        })?,
    );
    builder
        .build()
        .map_err(|error| InventoryError::InvalidGlob {
            pattern: pattern.into(),
            reason: error.to_string(),
        })
}

fn compile_modules(modules: &[ModuleSelector]) -> Result<Vec<CompiledModule>, InventoryError> {
    modules
        .iter()
        .map(|module| {
            let mut builder = GlobSetBuilder::new();
            for pattern in &module.files {
                builder.add(
                    Glob::new(pattern).map_err(|error| InventoryError::InvalidGlob {
                        pattern: pattern.clone(),
                        reason: error.to_string(),
                    })?,
                );
            }
            Ok(CompiledModule {
                id: module.id.clone(),
                matcher: builder
                    .build()
                    .map_err(|error| InventoryError::InvalidGlob {
                        pattern: module.files.join(","),
                        reason: error.to_string(),
                    })?,
            })
        })
        .collect()
}

fn manifest_rules(declarations: &[ClassDeclaration]) -> Vec<ClassRule> {
    declarations
        .iter()
        .enumerate()
        .map(|(index, declaration)| ClassRule {
            id: format!("manifest:inventory.classes[{index}]"),
            class: declaration.class,
            files: declaration.files.clone(),
            replaces: declaration.replaces,
        })
        .collect()
}

fn compile_rules(rules: &[ClassRule]) -> Result<Vec<CompiledRule>, InventoryError> {
    let mut compiled = Vec::new();
    for rule in rules {
        for pattern in &rule.files {
            compiled.push(CompiledRule {
                rule: rule.clone(),
                matcher: compile_glob(pattern)?,
                pattern: pattern.clone(),
            });
        }
    }
    Ok(compiled)
}

fn compile_generated(manifest: &WarrantManifest) -> Result<Vec<GeneratedPattern>, InventoryError> {
    let mut patterns = Vec::new();
    for (index, declaration) in manifest.inventory.generated.iter().enumerate() {
        for pattern in &declaration.files {
            patterns.push(GeneratedPattern {
                declaration: index,
                matcher: compile_glob(pattern)?,
                pattern: pattern.clone(),
                producer: declaration.producer.clone(),
                inputs: declaration.inputs.clone(),
                reproducible: declaration.reproducible,
            });
        }
    }
    Ok(patterns)
}

fn compile_vendored(manifest: &WarrantManifest) -> Result<Vec<VendoredPattern>, InventoryError> {
    let mut patterns = Vec::new();
    for (index, declaration) in manifest.inventory.vendored.iter().enumerate() {
        for pattern in &declaration.files {
            patterns.push(VendoredPattern {
                declaration: index,
                matcher: compile_glob(pattern)?,
                pattern: pattern.clone(),
                source: declaration.source.clone(),
                version: declaration.version.clone(),
                treatment: declaration.treatment.clone(),
            });
        }
    }
    Ok(patterns)
}

fn matching_rules<'a>(path: &str, rules: &'a [CompiledRule]) -> Vec<&'a CompiledRule> {
    rules
        .iter()
        .filter(|rule| rule.matcher.is_match(path))
        .collect()
}

fn choose_explicit<'a>(
    path: &str,
    default: InventoryClass,
    matches: &[&'a CompiledRule],
) -> Result<Option<&'a CompiledRule>, InventoryError> {
    if matches.is_empty() {
        return Ok(None);
    }
    let classes: BTreeSet<InventoryClass> = matches.iter().map(|rule| rule.rule.class).collect();
    if classes.len() > 1 {
        let mut rules: Vec<String> = matches.iter().map(|rule| rule.rule.id.clone()).collect();
        rules.sort();
        rules.dedup();
        return Err(InventoryError::ClassificationConflict {
            path: path.into(),
            rules,
        });
    }
    for matched in matches {
        if matched.rule.class == default || default == InventoryClass::Unknown {
            continue;
        }
        match matched.rule.replaces {
            None => {
                return Err(InventoryError::MissingDefaultReplacement {
                    path: path.into(),
                    default,
                    rule: matched.rule.id.clone(),
                });
            }
            Some(named) if named != default => {
                return Err(InventoryError::WrongDefaultReplacement {
                    path: path.into(),
                    expected: default,
                    named,
                    rule: matched.rule.id.clone(),
                });
            }
            Some(_) => {}
        }
    }
    let mut sorted = matches.to_vec();
    sorted.sort_by(|left, right| left.rule.id.cmp(&right.rule.id));
    let selected = sorted[0];
    Ok(Some(selected))
}

fn matching_generated<'a>(
    path: &str,
    patterns: &'a [GeneratedPattern],
) -> Vec<&'a GeneratedPattern> {
    let mut matches: Vec<_> = patterns
        .iter()
        .filter(|item| item.matcher.is_match(path))
        .collect();
    matches.dedup_by_key(|item| item.declaration);
    matches
}

fn matching_vendored<'a>(path: &str, patterns: &'a [VendoredPattern]) -> Vec<&'a VendoredPattern> {
    let mut matches: Vec<_> = patterns
        .iter()
        .filter(|item| item.matcher.is_match(path))
        .collect();
    matches.dedup_by_key(|item| item.declaration);
    matches
}

fn matching_modules(path: &str, modules: &[CompiledModule]) -> Vec<String> {
    let mut owners: Vec<String> = modules
        .iter()
        .filter(|module| module.matcher.is_match(path))
        .map(|module| module.id.clone())
        .collect();
    owners.sort();
    owners.dedup();
    owners
}

/// Integration defaults (spec 5.2). The build-output directory names are a built-in
/// only for files Git does not track: a tracked file under `build/` or `dist/` is
/// classified by its extension like any other, and a repository that commits build
/// output declares it in its manifest.
fn default_class(
    path: &str,
    enabled: EnabledIntegrations,
    untracked: bool,
) -> (InventoryClass, String, String) {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    let segments: Vec<&str> = lower.split('/').collect();
    let in_dir =
        |candidate: &str| segments[..segments.len().saturating_sub(1)].contains(&candidate);
    let known_output = untracked
        && ["target", "dist", "build", "node_modules", ".next"]
            .iter()
            .any(|part| in_dir(part));
    let class = if known_output {
        InventoryClass::BuildOutput
    } else if enabled.typescript && lower.ends_with(".d.ts") {
        InventoryClass::Schema
    } else if enabled.typescript
        && (name.ends_with(".test.ts")
            || name.ends_with(".spec.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".spec.tsx")
            || in_dir("__tests__"))
    {
        InventoryClass::Test
    } else if source_language(path, enabled).is_some() {
        InventoryClass::Source
    } else if in_dir("schemas") || in_dir("schema") {
        InventoryClass::Schema
    } else if in_dir("tests") {
        InventoryClass::Test
    } else if in_dir("migrations") || lower.ends_with(".sql") {
        InventoryClass::Migration
    } else if lower.ends_with(".sh") || in_dir("scripts") || in_dir("bin") {
        InventoryClass::Script
    } else if matches!(
        name,
        "package.json" | "tsconfig.json" | "pnpm-workspace.yaml" | "cargo.toml" | "warrant.yaml"
    ) || lower.ends_with(".config.js")
        || lower.ends_with(".config.ts")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".toml")
    {
        InventoryClass::Config
    } else if lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".txt") {
        InventoryClass::Doc
    } else if [
        ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".woff", ".woff2", ".mp3", ".mp4",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
    {
        InventoryClass::Asset
    } else {
        InventoryClass::Unknown
    };
    (
        class,
        format!("default:{}", class.as_str()),
        format!("matched {} classification default", class.as_str()),
    )
}

fn source_language(path: &str, enabled: EnabledIntegrations) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    if enabled.typescript
        && [".ts", ".tsx", ".js", ".jsx", ".mts", ".cts"]
            .iter()
            .any(|extension| lower.ends_with(extension))
    {
        Some("typescript")
    } else if enabled.rust && lower.ends_with(".rs") {
        Some("rust")
    } else {
        None
    }
}

fn language(path: &str, enabled: EnabledIntegrations) -> Option<String> {
    source_language(path, enabled).map(str::to_owned)
}

fn prefixed_git_oid(blob: &Option<String>) -> Result<Option<String>, InventoryError> {
    let Some(oid) = blob else {
        return Ok(None);
    };
    if oid.starts_with("sha1:") || oid.starts_with("sha256:") {
        return Ok(Some(oid.clone()));
    }
    let algorithm = match oid.len() {
        40 => "sha1",
        64 => "sha256",
        _ => {
            return Err(InventoryError::InvalidDeclaration {
                reason: format!("snapshot blob `{oid}` has no recognized object format"),
            });
        }
    };
    if !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(InventoryError::InvalidDeclaration {
            reason: format!("snapshot blob `{oid}` is not a hexadecimal object id"),
        });
    }
    Ok(Some(format!("{algorithm}:{oid}")))
}

fn blob_id(path: &Path) -> Result<String, InventoryError> {
    let bytes = if path.is_symlink() {
        fs::read_link(path).map(|target| target.to_string_lossy().into_owned().into_bytes())
    } else {
        fs::read(path)
    }
    .map_err(|error| io_error(path, error))?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(7 + digest.len() * 2);
    encoded.push_str("sha256:");
    for byte in digest {
        use fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn discover_units_from_paths(
    read: Reader<'_>,
    paths: &[String],
    captured: &[String],
) -> Result<Vec<Unit>, InventoryError> {
    let mut units = BTreeMap::<String, Unit>::new();
    for path in paths {
        if path.ends_with("/tsconfig.json") || path == "tsconfig.json" {
            units.insert(
                parent_string(path),
                Unit {
                    root: parent_string(path),
                    configuration: path.clone(),
                    by: "tsconfig".into(),
                },
            );
        }
    }
    discover_tsconfig_references(read, paths, &mut units)?;
    discover_javascript_units(read, paths, &mut units)?;
    discover_cargo_units(read, paths, captured, &mut units)?;
    Ok(units.into_values().collect())
}

fn discover_tsconfig_references(
    read: Reader<'_>,
    paths: &[String],
    units: &mut BTreeMap<String, Unit>,
) -> Result<(), InventoryError> {
    let available: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let mut pending: Vec<String> = units
        .values()
        .map(|unit| unit.configuration.clone())
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(configuration) = pending.pop() {
        if !seen.insert(configuration.clone()) {
            continue;
        }
        let value = read_tsconfig(read, &configuration)?;
        for reference in value
            .get("references")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|reference| reference.get("path").and_then(Value::as_str))
        {
            let joined = Path::new(&parent_string(&configuration)).join(reference);
            let Some(mut referenced) = normalize_relative(&joined) else {
                continue;
            };
            if !referenced.ends_with(".json") {
                referenced = format!("{referenced}/tsconfig.json");
            }
            if !available.contains(referenced.as_str()) {
                continue;
            }
            let unit_root = parent_string(&referenced);
            units.insert(
                unit_root.clone(),
                Unit {
                    root: unit_root,
                    configuration: referenced.clone(),
                    by: "tsconfig-reference".into(),
                },
            );
            pending.push(referenced);
        }
    }
    Ok(())
}

/// Captured configuration bytes; the snapshot's refusal is kept as its own error.
fn read_captured(read: Reader<'_>, path: &str) -> Result<Vec<u8>, InventoryError> {
    read(path).map_err(|error| InventoryError::Read {
        path: path.into(),
        code: error.code,
        reason: error.reason,
    })
}

fn read_tsconfig(read: Reader<'_>, configuration: &str) -> Result<Value, InventoryError> {
    let mut bytes = read_captured(read, configuration)?;
    json_strip_comments::strip_slice(&mut bytes).map_err(|error| {
        InventoryError::InvalidDeclaration {
            reason: format!("invalid `{configuration}`: {error}"),
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|error| InventoryError::InvalidDeclaration {
        reason: format!("invalid `{configuration}`: {error}"),
    })
}

fn normalize_relative(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    })
}

fn discover_javascript_units(
    read: Reader<'_>,
    paths: &[String],
    units: &mut BTreeMap<String, Unit>,
) -> Result<(), InventoryError> {
    let package_files: Vec<&String> = paths
        .iter()
        .filter(|path| path.ends_with("package.json"))
        .collect();
    for package_file in &package_files {
        let bytes = read_captured(read, package_file)?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|error| InventoryError::InvalidDeclaration {
                reason: format!("invalid `{package_file}`: {error}"),
            })?;
        let package_root = parent_string(package_file);
        units.entry(package_root.clone()).or_insert_with(|| Unit {
            root: package_root.clone(),
            configuration: (*package_file).clone(),
            by: "package-json".into(),
        });
        if let Some(workspaces) = value.get("workspaces") {
            let patterns = workspace_patterns(workspaces);
            add_workspace_packages(
                &package_root,
                &patterns,
                &package_files,
                units,
                "package-workspace",
            )?;
        }
    }
    for workspace_file in paths
        .iter()
        .filter(|path| path.ends_with("pnpm-workspace.yaml"))
    {
        let bytes = String::from_utf8(read_captured(read, workspace_file)?).map_err(|_| {
            InventoryError::InvalidDeclaration {
                reason: format!("invalid `{workspace_file}`: not UTF-8"),
            }
        })?;
        let value: serde_json::Value =
            serde_saphyr::from_str(&bytes).map_err(|error| InventoryError::InvalidDeclaration {
                reason: format!("invalid `{workspace_file}`: {error}"),
            })?;
        let patterns = value
            .get("packages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        add_workspace_packages(
            &parent_string(workspace_file),
            &patterns,
            &package_files,
            units,
            "pnpm-workspace",
        )?;
    }
    Ok(())
}

fn workspace_patterns(value: &Value) -> Vec<String> {
    let array = value
        .as_array()
        .or_else(|| value.get("packages").and_then(Value::as_array));
    array
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn add_workspace_packages(
    workspace_root: &str,
    patterns: &[String],
    package_files: &[&String],
    units: &mut BTreeMap<String, Unit>,
    by: &str,
) -> Result<(), InventoryError> {
    for pattern in patterns {
        let qualified = if workspace_root == "." {
            pattern.clone()
        } else {
            format!("{workspace_root}/{pattern}")
        };
        let matcher = compile_glob(&qualified)?;
        for package_file in package_files {
            let package_root = parent_string(package_file);
            if matcher.is_match(&package_root) {
                units.entry(package_root.clone()).or_insert_with(|| Unit {
                    root: package_root,
                    configuration: (*package_file).clone(),
                    by: by.into(),
                });
            }
        }
    }
    Ok(())
}

/// Cargo units from `cargo metadata` over a copy of the captured Cargo inputs, never the
/// live tree. Rust sources are copied empty: target discovery needs them to exist and
/// `--no-deps` never reads them.
fn discover_cargo_units(
    read: Reader<'_>,
    paths: &[String],
    captured: &[String],
    units: &mut BTreeMap<String, Unit>,
) -> Result<(), InventoryError> {
    for manifest in paths.iter().filter(|path| path.ends_with("Cargo.toml")) {
        let unit_root = parent_string(manifest);
        units.entry(unit_root.clone()).or_insert_with(|| Unit {
            root: unit_root,
            configuration: manifest.clone(),
            by: "cargo-manifest".into(),
        });
    }
    if !captured.iter().any(|path| path == "Cargo.toml") {
        return Ok(());
    }
    let temporary = tempfile::tempdir().map_err(|error| io_error("temporary directory", error))?;
    let copy = temporary
        .path()
        .canonicalize()
        .map_err(|error| io_error("temporary directory", error))?;
    for path in captured {
        if matches!(path.as_str(), "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml")
            || path.ends_with("/Cargo.toml")
        {
            write_captured(&copy, path, &read_captured(read, path)?, None)?;
        } else if path.ends_with(".rs") {
            write_captured(&copy, path, &[], None)?;
        }
    }
    let mut command = MetadataCommand::new();
    command
        .manifest_path(copy.join("Cargo.toml"))
        .current_dir(&copy)
        .no_deps()
        .other_options(["--locked".into()]);
    let has_lock = captured.iter().any(|path| path == "Cargo.lock");
    let metadata = match command.exec() {
        Ok(metadata) => metadata,
        Err(error) if has_lock => {
            let prefix = format!("{}/", copy.display());
            return Err(InventoryError::InvalidDeclaration {
                reason: format!(
                    "cargo metadata failed for `Cargo.toml`: {}",
                    error.to_string().replace(&prefix, "")
                ),
            });
        }
        // Do not let discovery create a lockfile in the source tree. Manifest
        // roots still give conservative unit boundaries when no lock exists.
        Err(_) => return Ok(()),
    };
    for package in metadata.packages {
        let manifest_path = PathBuf::from(package.manifest_path.as_std_path());
        let relative = relative_path(&copy, &manifest_path)?;
        if !paths.contains(&relative) {
            continue;
        }
        let unit_root = parent_string(&relative);
        units.entry(unit_root.clone()).or_insert(Unit {
            root: unit_root,
            configuration: relative,
            by: "cargo-metadata".into(),
        });
    }
    Ok(())
}

fn parent_string(path: &str) -> String {
    Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| ".".into())
}

fn unit_for<'a>(path: &str, units: &'a [Unit], source_language: Option<&str>) -> Option<&'a Unit> {
    let contains = |unit: &&Unit| {
        unit.root == "." || path == unit.root || path.starts_with(&format!("{}/", unit.root))
    };
    let depth = |unit: &&Unit| unit.root.matches('/').count() + usize::from(unit.root != ".");
    if source_language == Some("typescript") {
        units
            .iter()
            .filter(contains)
            .filter(|unit| is_tsconfig(&unit.configuration))
            .max_by_key(depth)
            .or_else(|| units.iter().filter(contains).max_by_key(depth))
    } else {
        units.iter().filter(contains).max_by_key(depth)
    }
}

fn is_tsconfig(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("tsconfig") && name.ends_with(".json"))
}

fn discover_package_entrypoints(
    read: Reader<'_>,
    paths: &[String],
) -> Result<BTreeMap<String, Vec<Entrypoint>>, InventoryError> {
    let mut found = BTreeMap::<String, Vec<Entrypoint>>::new();
    for package_file in paths.iter().filter(|path| path.ends_with("package.json")) {
        let value: Value =
            serde_json::from_slice(&read_captured(read, package_file)?).map_err(|error| {
                InventoryError::InvalidDeclaration {
                    reason: format!("invalid `{package_file}`: {error}"),
                }
            })?;
        let package_root = parent_string(package_file);
        collect_field_targets(
            &value,
            "bin",
            "package-bin",
            &package_root,
            package_file,
            &mut found,
        );
        collect_field_targets(
            &value,
            "main",
            "package-main",
            &package_root,
            package_file,
            &mut found,
        );
        collect_field_targets(
            &value,
            "module",
            "package-main",
            &package_root,
            package_file,
            &mut found,
        );
        collect_field_targets(
            &value,
            "exports",
            "package-export",
            &package_root,
            package_file,
            &mut found,
        );
    }
    Ok(found)
}

fn collect_field_targets(
    package: &Value,
    field: &str,
    kind: &str,
    package_root: &str,
    package_file: &str,
    found: &mut BTreeMap<String, Vec<Entrypoint>>,
) {
    let Some(value) = package.get(field) else {
        return;
    };
    let mut targets = Vec::new();
    string_leaves(value, &mut targets);
    for target in targets {
        if !target.starts_with('.') && target.contains(':') {
            continue;
        }
        let target = target.trim_start_matches("./");
        let path = if package_root == "." {
            target.into()
        } else {
            format!("{package_root}/{target}")
        };
        found.entry(path).or_default().push(Entrypoint {
            kind: kind.into(),
            basis: "declared".into(),
            by: format!("{package_file}:{field}"),
        });
    }
}

fn string_leaves<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
    match value {
        Value::String(value) => found.push(value),
        Value::Array(values) => values.iter().for_each(|value| string_leaves(value, found)),
        Value::Object(values) => values
            .values()
            .for_each(|value| string_leaves(value, found)),
        _ => {}
    }
}

fn entrypoints_for(
    path: &str,
    manifest: &WarrantManifest,
    package: &BTreeMap<String, Vec<Entrypoint>>,
) -> Vec<Entrypoint> {
    let mut entrypoints = package.get(path).cloned().unwrap_or_default();
    entrypoints.extend(
        manifest
            .entrypoints
            .iter()
            .filter(|item| item.path == path)
            .map(|item| Entrypoint {
                kind: item.kind.clone(),
                basis: "declared".into(),
                by: "manifest:entrypoints".into(),
            }),
    );
    entrypoints.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.by.cmp(&right.by)));
    entrypoints.dedup_by(|left, right| left.kind == right.kind && left.by == right.by);
    entrypoints
}

fn alias_tables(
    read: Reader<'_>,
    paths: &[String],
    units: &[Unit],
) -> Result<Vec<UnitAliasTable>, InventoryError> {
    let available: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let mut rows = Vec::with_capacity(units.len());
    for unit in units {
        let mut alias_table = None;
        if is_tsconfig(&unit.configuration) && !unit.configuration.is_empty() {
            let value = read_tsconfig(read, &unit.configuration)?;
            if value
                .get("compilerOptions")
                .and_then(|options| options.get("paths"))
                .is_some_and(Value::is_object)
            {
                alias_table = Some(unit.configuration.clone());
            }
        }
        if alias_table.is_none() {
            let package = if unit.root == "." {
                "package.json".into()
            } else {
                format!("{}/package.json", unit.root)
            };
            if available.contains(package.as_str()) {
                let value: Value = serde_json::from_slice(&read_captured(read, &package)?)
                    .map_err(|error| InventoryError::InvalidDeclaration {
                        reason: format!("invalid `{package}`: {error}"),
                    })?;
                if value.get("exports").is_some() {
                    alias_table = Some(package);
                }
            }
        }
        rows.push(UnitAliasTable {
            unit: unit.root.clone(),
            alias_table,
            by: unit.by.clone(),
        });
    }
    rows.sort_by(|left, right| left.unit.cmp(&right.unit));
    Ok(rows)
}

/// The snapshot's reason on an untracked nested repository it recorded as a gitlink.
const UNDECLARED_NESTED_REPOSITORY: &str = "undeclared-nested-repository";

/// The reason on a declared generated path the snapshot does not contain.
const GENERATED_ABSENT: &str = "generated-absent";

fn add_absent_generated(
    entries: &mut Vec<InventoryEntry>,
    patterns: &[GeneratedPattern],
    captured: &[&str],
) -> Result<Vec<GeneratedAbsent>, InventoryError> {
    let mut absent = Vec::new();
    for item in patterns {
        if captured.iter().any(|path| item.matcher.is_match(path)) {
            continue;
        }
        if !is_literal(&item.pattern) {
            absent.push(GeneratedAbsent {
                declaration: item.pattern.clone(),
                producer: item.producer.clone(),
            });
            continue;
        }
        let matches = matching_generated(&item.pattern, patterns);
        if matches.len() > 1 {
            let mut rules: Vec<_> = matches
                .iter()
                .map(|matched| format!("manifest:inventory.generated[{}]", matched.declaration))
                .collect();
            rules.sort();
            return Err(InventoryError::ClassificationConflict {
                path: item.pattern.clone(),
                rules,
            });
        }
        // The path is listed as an ignored entry; keep that entry and record the
        // absence with its producer in the summary rather than duplicating the path.
        if entries.iter().any(|entry| entry.path == item.pattern) {
            absent.push(GeneratedAbsent {
                declaration: item.pattern.clone(),
                producer: item.producer.clone(),
            });
            continue;
        }
        entries.push(InventoryEntry {
            path: item.pattern.clone(),
            blob: None,
            class: InventoryClass::Generated,
            language: None,
            unit: None,
            module: None,
            by: "manifest:inventory.generated".into(),
            reason: GENERATED_ABSENT.into(),
            entrypoints: Vec::new(),
            unread: None,
            generated_by: Some(GeneratedBy {
                producer: item.producer.clone(),
                reproducible: item.reproducible,
                inputs: item.inputs.clone(),
            }),
            vendored_from: None,
        });
    }
    absent.sort_by(|left, right| left.declaration.cmp(&right.declaration));
    Ok(absent)
}

fn summarize(
    entries: &[InventoryEntry],
    kind: &SnapshotKind,
    unit_aliases: Vec<UnitAliasTable>,
    generated_absent: Vec<GeneratedAbsent>,
) -> InventorySummary {
    // Spec 4.3: only a worktree capture sees ignored files; for an index, commit or tree
    // the count is unknown (null), which is not the same claim as zero.
    let ignored_files = (*kind == SnapshotKind::Worktree).then(|| {
        entries
            .iter()
            .filter(|entry| entry.class == InventoryClass::Ignored)
            .count() as u64
    });
    // `files` counts what the snapshot holds: an ignored path is outside it, and a
    // declared generated file that is absent is listed for its producer, not present.
    let files = entries
        .iter()
        .filter(|entry| entry.class != InventoryClass::Ignored && entry.reason != GENERATED_ABSENT)
        .count() as u64;
    let mut summary = InventorySummary {
        files,
        ignored_files,
        unit_aliases,
        generated_absent,
        ..InventorySummary::default()
    };
    for entry in entries {
        *summary
            .by_class
            .entry(entry.class.as_str().into())
            .or_default() += 1;
        match entry.class {
            InventoryClass::Unread => summary.unread.push(UnreadPath {
                path: entry.path.clone(),
                reason: entry.unread.clone().unwrap_or_else(|| entry.reason.clone()),
            }),
            InventoryClass::Unknown => summary.unknown.push(entry.path.clone()),
            InventoryClass::Submodule => summary.submodules.push(Submodule {
                path: entry.path.clone(),
                commit: entry.blob.clone().unwrap_or_else(|| "unknown".into()),
            }),
            InventoryClass::Source if entry.module.is_none() => {
                summary.unowned_source.push(entry.path.clone())
            }
            _ => {}
        }
    }
    summary
}

/// Write one captured path into the verification copy with its recorded mode.
fn write_captured(
    root: &Path,
    path: &str,
    bytes: &[u8],
    mode: Option<&str>,
) -> Result<(), InventoryError> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    }
    #[cfg(unix)]
    if mode == Some("120000") {
        use std::os::unix::ffi::OsStrExt;
        return std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(bytes), &target)
            .map_err(|error| io_error(path, error));
    }
    fs::write(&target, bytes).map_err(|error| io_error(path, error))?;
    #[cfg(unix)]
    if mode == Some("100755") {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
            .map_err(|error| io_error(path, error))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

fn is_literal(pattern: &str) -> bool {
    !pattern
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'{' | b'!'))
}

fn io_error(path: impl AsRef<Path>, error: impl fmt::Display) -> InventoryError {
    InventoryError::Io {
        path: path.as_ref().to_string_lossy().into_owned(),
        reason: error.to_string(),
    }
}
