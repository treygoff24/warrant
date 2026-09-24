use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use warrant_core::manifest::WarrantManifest;
use warrant_core::nouns::{
    GeneratedAbsent, GeneratedDrift, InventoryClass, InventoryEntry, InventorySummary,
    SnapshotKind, SnapshotManifest,
};
use warrant_inventory::{
    BuildConfig, BuiltInventory, CapturedSnapshot, ClassRule, GeneratedIssue, GeneratedIssueCode,
    InventoryError, ModuleSelector, ReadError, build, discover_units, record_verification,
    verify_generated,
};

fn write(root: &Path, path: &str, contents: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().expect("file parent")).expect("create parent");
    fs::write(target, contents).expect("write fixture file");
}

fn snapshot_entry(path: &str, class: InventoryClass, blob: Option<&str>) -> InventoryEntry {
    InventoryEntry {
        path: path.into(),
        blob: blob.map(str::to_owned),
        class,
        language: None,
        unit: None,
        module: None,
        by: "snapshot".into(),
        reason: "captured fixture".into(),
        entrypoints: Vec::new(),
        unread: (class == InventoryClass::Unread).then(|| "oversize".into()),
        generated_by: None,
        vendored_from: None,
    }
}

fn snapshot(root: &Path) -> Vec<InventoryEntry> {
    fn collect(root: &Path, directory: &Path, entries: &mut Vec<InventoryEntry>) {
        for item in fs::read_dir(directory).expect("read fixture directory") {
            let item = item.expect("read fixture entry");
            let path = item.path();
            if item.file_type().expect("fixture file type").is_dir() {
                collect(root, &path, entries);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("fixture path below root")
                    .to_string_lossy()
                    .replace('\\', "/");
                // Inventory consumes captured blobs and read failures, not live file bytes.
                let entry = match fs::read(&path) {
                    Ok(bytes) => {
                        let mut hash = Sha256::new();
                        hash.update(format!("blob {}\0", bytes.len()));
                        hash.update(bytes);
                        let digest: String = hash
                            .finalize()
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect();
                        snapshot_entry(
                            &relative,
                            InventoryClass::Unknown,
                            Some(&format!("sha256:{digest}")),
                        )
                    }
                    Err(error) => {
                        let mut entry = snapshot_entry(&relative, InventoryClass::Unread, None);
                        entry.unread = Some(error.to_string());
                        entry
                    }
                };
                entries.push(entry);
            }
        }
    }

    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

/// The manifest of a worktree capture; fixture listings stand in for its entries.
fn worktree_manifest() -> SnapshotManifest {
    SnapshotManifest::new(
        "sha1:0000000000000000000000000000000000000001".into(),
        SnapshotKind::Worktree,
        "sha1:0000000000000000000000000000000000000002".into(),
        "2026-09-24T00:00:00Z".into(),
    )
}

/// Build with a reader over the fixture directory, as a worktree snapshot reads it.
/// Fixture listings have no Git, so every entry is treated as tracked.
fn build_on_disk(
    root: &Path,
    listing: &[InventoryEntry],
    manifest: &WarrantManifest,
    config: &BuildConfig,
) -> Result<BuiltInventory, InventoryError> {
    build_on_disk_with_untracked(root, listing, &BTreeSet::new(), manifest, config)
}

/// `verify_generated` over the listing, reading fixture bytes as the snapshot would.
fn verify_on_disk(
    root: &Path,
    listing: &[InventoryEntry],
    manifest: &WarrantManifest,
) -> Result<Vec<warrant_inventory::GeneratedIssue>, InventoryError> {
    let read = |path: &str| {
        fs::read(root.join(path)).map_err(|error| ReadError {
            code: "io".into(),
            reason: error.to_string(),
        })
    };
    let mode = |_: &str| Some("100644".to_owned());
    verify_generated(
        CapturedSnapshot {
            manifest: &worktree_manifest(),
            entries: listing,
            read: &read,
            untracked: &BTreeSet::new(),
        },
        &mode,
        manifest,
    )
}

/// As `build_on_disk`, with the named paths untracked in the worktree.
fn build_on_disk_with_untracked(
    root: &Path,
    listing: &[InventoryEntry],
    untracked: &BTreeSet<String>,
    manifest: &WarrantManifest,
    config: &BuildConfig,
) -> Result<BuiltInventory, InventoryError> {
    let read = |path: &str| {
        fs::read(root.join(path)).map_err(|error| ReadError {
            code: "io".into(),
            reason: error.to_string(),
        })
    };
    build(
        root,
        CapturedSnapshot {
            manifest: &worktree_manifest(),
            entries: listing,
            read: &read,
            untracked,
        },
        manifest,
        config,
    )
}

fn manifest(inventory: &str) -> WarrantManifest {
    WarrantManifest::parse(&format!(
        r#"
schema_version: warrant.manifest/1
integrations:
  lang-ts:
    enabled: true
  lang-rust:
    enabled: true
inventory:
{inventory}
"#
    ))
    .expect("valid fixture manifest")
}

fn empty_manifest() -> WarrantManifest {
    manifest("  unknown: report")
}

#[test]
fn overlapping_module_selectors_are_an_error() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/shared.ts", "export const value = 1;\n");

    let first = ModuleSelector {
        id: "first".into(),
        files: vec!["src/**".into()],
    };
    let second = ModuleSelector {
        id: "second".into(),
        files: vec!["src/shared.ts".into()],
    };
    for modules in [vec![first.clone(), second.clone()], vec![second, first]] {
        let error = build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &BuildConfig {
                modules,
                ..BuildConfig::default()
            },
        )
        .expect_err("overlap must not be resolved by declaration order");
        // Modules come from policy, which the M0 CLI does not load; the code is pinned here.
        assert_eq!(error.code(), "ownership-overlap");
        assert!(matches!(
            error,
            InventoryError::OwnershipOverlap { path, modules }
                if path == "src/shared.ts" && modules == ["first", "second"]
        ));
    }
}

#[test]
fn nested_repository_is_rejected() {
    for git_is_file in [false, true] {
        let root = tempdir().expect("temporary repository");
        if git_is_file {
            write(root.path(), "vendor/nested/.git", "gitdir: elsewhere\n");
        } else {
            write(
                root.path(),
                "vendor/nested/.git/HEAD",
                "ref: refs/heads/main\n",
            );
        }
        write(root.path(), "vendor/nested/src/b.ts", "export {};\n");
        let error = build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &BuildConfig::default(),
        )
        .expect_err("nested repository has two identities");
        assert!(
            matches!(error, InventoryError::NestedRepository { path } if path == "vendor/nested")
        );
    }
}

/// The worktree snapshot records an untracked nested repository as a `submodule`-class
/// entry (the gitlink Git would stage) with reason `undeclared-nested-repository`; an
/// unborn one carries no commit. Neither is a declared submodule (spec 5.6).
#[test]
fn snapshot_recorded_undeclared_nested_repository_is_rejected() {
    let listing_with = |reason: &str| {
        let root = tempdir().expect("temporary repository");
        write(root.path(), "src/a.ts", "export {};\n");
        let mut listing = snapshot(root.path());
        for (path, blob) in [("vendor/zeta", Some("sha1:abc")), ("vendor/alpha", None)] {
            let mut entry = snapshot_entry(path, InventoryClass::Submodule, blob);
            entry.reason = reason.into();
            entry.unread = Some("submodule-not-descended".into());
            listing.push(entry);
        }
        (root, listing)
    };

    let (root, listing) = listing_with("undeclared-nested-repository");
    let result = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    );
    assert!(
        matches!(&result, Err(InventoryError::NestedRepository { path }) if path == "vendor/alpha"),
        "undeclared nested repository built: {result:?}"
    );

    let (root, listing) = listing_with("captured");
    let built = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("declared submodules build");
    let submodules: Vec<_> = built
        .document
        .summary
        .submodules
        .iter()
        .map(|submodule| submodule.path.as_str())
        .collect();
    assert_eq!(submodules, ["vendor/alpha", "vendor/zeta"]);
}

#[test]
fn declared_submodule_contents_are_not_first_party() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(
        root.path(),
        "vendor/sub/.git/HEAD",
        "ref: refs/heads/main\n",
    );
    write(root.path(), "vendor/sub/src/b.ts", "export {};\n");
    write(
        root.path(),
        "vendor/sub/package.json",
        "not first-party config",
    );
    write(root.path(), "vendor/submarine.ts", "export {};\n");
    let mut listing = snapshot(root.path());
    listing.push(snapshot_entry(
        "vendor/sub",
        InventoryClass::Submodule,
        Some("sha1:abc"),
    ));
    let built = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("declared submodule is allowed");
    let paths: Vec<_> = built
        .document
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(paths, ["vendor/sub", "vendor/submarine.ts"]);
    assert_eq!(built.document.summary.submodules[0].path, "vendor/sub");
    assert_eq!(
        built.document.summary.unowned_source,
        ["vendor/submarine.ts"]
    );
}

#[test]
fn classification_defaults_and_snapshot_exclusions_preserve_classes() {
    let root = tempdir().expect("temporary repository");
    for (path, contents) in [
        ("src/main.ts", "export {};\n"),
        ("src/main.test.ts", "test('x', () => {});\n"),
        ("package.json", "{}\n"),
        ("scripts/release.sh", "exit 0\n"),
        ("migrations/001.sql", "select 1;\n"),
        ("types/api.d.ts", "export {};\n"),
        ("generated/client.ts", "generated\n"),
        ("vendor/library.ts", "vendored\n"),
        ("public/logo.png", "image\n"),
        ("README.md", "docs\n"),
        ("dist/app.js", "built\n"),
        ("legacy.pl", "unknown\n"),
    ] {
        write(root.path(), path, contents);
    }
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/**"]
      producer: "generate-client"
      reproducible: false
  vendored:
    - files: ["vendor/**"]
      source: "https://example.invalid/library"
      version: "1.2.3"
      treatment: "checked-as-third-party"
"#,
    );
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "application".into(),
            files: vec!["src/**".into()],
        }],
        ..BuildConfig::default()
    };
    let mut listing = snapshot(root.path());
    listing.extend([
        snapshot_entry(
            "third-party/old",
            InventoryClass::Submodule,
            Some("sha1:abc"),
        ),
        snapshot_entry("ignored/cache.bin", InventoryClass::Ignored, None),
        snapshot_entry("large/data.bin", InventoryClass::Unread, None),
    ]);

    let built = build_on_disk(root.path(), &listing, &manifest, &config).expect("inventory builds");
    let classes: BTreeMap<_, _> = built
        .document
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.class))
        .collect();
    assert_eq!(classes["src/main.ts"], InventoryClass::Source);
    assert_eq!(classes["src/main.test.ts"], InventoryClass::Test);
    assert_eq!(classes["package.json"], InventoryClass::Config);
    assert_eq!(classes["scripts/release.sh"], InventoryClass::Script);
    assert_eq!(classes["migrations/001.sql"], InventoryClass::Migration);
    assert_eq!(classes["types/api.d.ts"], InventoryClass::Schema);
    assert_eq!(classes["generated/client.ts"], InventoryClass::Generated);
    assert_eq!(classes["vendor/library.ts"], InventoryClass::Vendored);
    assert_eq!(classes["public/logo.png"], InventoryClass::Asset);
    assert_eq!(classes["README.md"], InventoryClass::Doc);
    // Spec 5.2: a tracked file under dist/ is classified by its extension.
    assert_eq!(classes["dist/app.js"], InventoryClass::Source);
    assert_eq!(classes["third-party/old"], InventoryClass::Submodule);
    assert_eq!(classes["ignored/cache.bin"], InventoryClass::Ignored);
    assert_eq!(classes["legacy.pl"], InventoryClass::Unknown);
    assert_eq!(classes["large/data.bin"], InventoryClass::Unread);

    let generated = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "generated/client.ts")
        .expect("generated entry");
    assert_eq!(
        generated
            .generated_by
            .as_ref()
            .expect("generated provenance")
            .producer,
        "generate-client"
    );
    let vendored = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "vendor/library.ts")
        .expect("vendored entry");
    assert_eq!(
        vendored
            .vendored_from
            .as_ref()
            .expect("vendored provenance")
            .version,
        "1.2.3"
    );
}

#[test]
fn ignored_tsconfig_and_its_reference_do_not_create_units() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "tsconfig.json",
        r#"{"references":[{"path":"node_modules/x"}]}"#,
    );
    write(
        root.path(),
        "node_modules/x/tsconfig.json",
        "{\"compilerOptions\": {},}\n",
    );
    let mut listing = snapshot(root.path());
    listing
        .iter_mut()
        .find(|entry| entry.path == "node_modules/x/tsconfig.json")
        .expect("dependency tsconfig")
        .class = InventoryClass::Ignored;

    let built = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("ignored dependency tsconfig is not parsed");
    assert_eq!(built.units.len(), 1);
    assert_eq!(built.units[0].root, ".");
    assert_eq!(built.units[0].configuration, "tsconfig.json");
    assert_eq!(built.document.summary.ignored_files, Some(1));
}

#[test]
fn first_party_tsconfig_accepts_comments_and_trailing_commas() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "tsconfig.json",
        "{\n  // compiler alias\n  \"compilerOptions\": {\"paths\": {\"@/*\": [\"src/*\",],},},\n}\n",
    );
    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("TypeScript JSONC is accepted");
    assert_eq!(built.units.len(), 1);
    assert_eq!(built.units[0].configuration, "tsconfig.json");
    assert_eq!(
        built.document.summary.unit_aliases[0]
            .alias_table
            .as_deref(),
        Some("tsconfig.json")
    );
}

#[test]
fn invalid_first_party_tsconfig_names_its_path() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "tsconfig.json", "{\"compilerOptions\": }\n");
    let error = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect_err("invalid JSONC must fail");
    assert!(matches!(
        error,
        InventoryError::InvalidDeclaration { reason }
            if reason.starts_with("invalid `tsconfig.json`:")
    ));
}

#[test]
fn excluded_declarations_do_not_supply_units_or_entrypoints() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "package.json", r#"{"main":"src/index.ts"}"#);
    write(root.path(), "src/index.ts", "export {};\n");
    write(root.path(), "dist/tsconfig.json", "not JSON");
    write(root.path(), "dist/package.json", "not JSON");
    write(root.path(), "vendor/tsconfig.json", "not JSON");
    write(root.path(), "vendor/package.json", "not JSON");
    write(root.path(), "node_modules/x/package.json", "not JSON");
    let mut listing = snapshot(root.path());
    listing
        .iter_mut()
        .find(|entry| entry.path == "node_modules/x/package.json")
        .expect("ignored dependency package")
        .class = InventoryClass::Ignored;
    let manifest = manifest(
        "  vendored:\n    - files: ['vendor/**']\n      source: vendor\n      version: '1'\n",
    );

    // Untracked build output is excluded from discovery, so its declarations are not parsed.
    let untracked: BTreeSet<String> = ["dist/tsconfig.json", "dist/package.json"]
        .map(String::from)
        .into();
    let built = build_on_disk_with_untracked(
        root.path(),
        &listing,
        &untracked,
        &manifest,
        &BuildConfig::default(),
    )
    .expect("excluded declarations are not parsed");
    assert_eq!(built.units.len(), 1);
    assert_eq!(built.units[0].configuration, "package.json");
    let source = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "src/index.ts")
        .expect("package main target");
    assert_eq!(source.entrypoints.len(), 1);
    assert_eq!(source.entrypoints[0].kind, "package-main");

    // Tracked, the same files are first-party configuration (spec 5.2) and are parsed.
    let error = build_on_disk(root.path(), &listing, &manifest, &BuildConfig::default())
        .expect_err("a tracked malformed tsconfig is a first-party declaration");
    assert!(
        matches!(&error, InventoryError::InvalidDeclaration { reason }
            if reason.contains("dist/tsconfig.json")),
        "{error:?}"
    );
}

#[test]
fn colocated_test_keeps_its_class_and_owner() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/service.test.ts", "export {};\n");
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "service".into(),
            files: vec!["src/**".into()],
        }],
        ..BuildConfig::default()
    };

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &config,
    )
    .expect("inventory builds");
    let entry = &built.document.entries[0];
    assert_eq!(entry.class, InventoryClass::Test);
    assert_eq!(entry.module.as_deref(), Some("service"));
}

#[test]
fn source_defaults_only_apply_for_enabled_integrations() {
    let root = tempdir().expect("temporary repository");
    for path in [
        "types/disabled.d.ts",
        "src/value.test.ts",
        "src/value.spec.tsx",
        "__tests__/value.ts",
    ] {
        write(root.path(), path, "export {};\n");
    }
    let manifest =
        WarrantManifest::parse("schema_version: warrant.manifest/1\n").expect("valid manifest");

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    for entry in &built.document.entries {
        assert_eq!(entry.class, InventoryClass::Unknown, "{}", entry.path);
    }
}

#[test]
fn unowned_bin_source_keeps_integration_defaults() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "package.json", "{}");
    let sources = [
        "crates/tool/src/bin/helper.rs",
        "migrations/change.ts",
        "schemas/model.rs",
        "scripts/tool.ts",
        "src/bin/cli.ts",
        "tests/helper.rs",
        "vite.config.ts",
    ];
    for path in sources {
        write(root.path(), path, "// source\n");
    }
    write(root.path(), "scripts/release.sh", "exit 0\n");
    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    for path in sources {
        let entry = built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("source entry");
        assert_eq!(entry.class, InventoryClass::Source, "{path}");
        assert_eq!(entry.module, None);
        assert_eq!(entry.by, "default:source");
    }
    assert_eq!(built.document.summary.unowned_source, sources);
    assert_eq!(
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == "scripts/release.sh")
            .expect("shell script")
            .class,
        InventoryClass::Script
    );
}

#[test]
fn conflicting_class_rules_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let first = ClassRule {
        id: "doc-rule".into(),
        class: InventoryClass::Doc,
        files: vec!["src/**".into()],
        replaces: Some(InventoryClass::Source),
    };
    let second = ClassRule {
        id: "script-rule".into(),
        class: InventoryClass::Script,
        files: vec!["src/value.ts".into()],
        replaces: Some(InventoryClass::Source),
    };

    for rules in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        let error = build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &BuildConfig {
                class_rules: rules,
                ..BuildConfig::default()
            },
        )
        .expect_err("conflicting classes must fail");
        assert!(matches!(
            error,
            InventoryError::ClassificationConflict { path, rules }
                if path == "src/value.ts" && rules == ["doc-rule", "script-rule"]
        ));
    }
}

#[test]
fn overlapping_generated_declarations_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/client.ts", "generated\n");
    let first = "    - files: [\"generated/**\"]\n      producer: first\n";
    let second = "    - files: [\"generated/client.ts\"]\n      producer: second\n";
    for declarations in [format!("{first}{second}"), format!("{second}{first}")] {
        let error = build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &manifest(&format!("  generated:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("overlapping producers must fail");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "generated/client.ts" && rules == ["manifest:inventory.generated[0]", "manifest:inventory.generated[1]"])
        );
    }
}

#[test]
fn overlapping_vendored_declarations_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "vendor/client.ts", "vendored\n");
    let first = "    - files: [\"vendor/**\"]\n      source: first\n      version: '1'\n";
    let second = "    - files: [\"vendor/client.ts\"]\n      source: second\n      version: '2'\n";
    for declarations in [format!("{first}{second}"), format!("{second}{first}")] {
        let error = build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &manifest(&format!("  vendored:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("overlapping vendors must fail");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "vendor/client.ts" && rules == ["manifest:inventory.vendored[0]", "manifest:inventory.vendored[1]"])
        );
    }
}

#[test]
fn duplicate_absent_generated_declarations_are_errors() {
    let root = tempdir().expect("temporary repository");
    for producers in [["first", "second"], ["second", "first"]] {
        let declarations: String = producers
            .iter()
            .map(|producer| {
                format!("    - files: [\"generated/missing.ts\"]\n      producer: {producer}\n")
            })
            .collect();
        let error = build_on_disk(
            root.path(),
            &[],
            &manifest(&format!("  generated:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("an absent path cannot have conflicting producers");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "generated/missing.ts" && rules == ["manifest:inventory.generated[0]", "manifest:inventory.generated[1]"])
        );
    }
}

#[test]
fn overlapping_patterns_within_one_generated_declaration_are_not_conflicts() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/client.ts", "generated\n");
    let manifest = manifest(
        "  generated:\n    - files: ['generated/**', 'generated/client.ts', 'absent.ts', 'absent.ts']\n      producer: generate\n",
    );
    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("one declaration owns both paths");
    assert_eq!(built.document.entries.len(), 2);
    // The absent literal is listed for its producer but is not a file the snapshot holds.
    assert_eq!(built.document.summary.files, 1);
    assert_eq!(built.document.entries[0].path, "absent.ts");
    assert_eq!(built.document.entries[1].path, "generated/client.ts");
}

#[test]
fn explicit_override_must_name_the_default_it_replaces() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let make_config = |replaces| BuildConfig {
        class_rules: vec![ClassRule {
            id: "migration".into(),
            class: InventoryClass::Migration,
            files: vec!["src/value.ts".into()],
            replaces,
        }],
        ..BuildConfig::default()
    };

    assert!(matches!(
        build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &make_config(None)
        ),
        Err(InventoryError::MissingDefaultReplacement {
            default: InventoryClass::Source,
            ..
        })
    ));
    assert!(matches!(
        build_on_disk(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &make_config(Some(InventoryClass::Test))
        ),
        Err(InventoryError::WrongDefaultReplacement {
            expected: InventoryClass::Source,
            named: InventoryClass::Test,
            ..
        })
    ));
    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &make_config(Some(InventoryClass::Source)),
    )
    .expect("named replacement is valid");
    assert_eq!(built.document.entries[0].class, InventoryClass::Migration);
}

#[test]
fn manifest_class_override_names_the_replaced_default() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let manifest = manifest(
        r#"  classes:
    - class: migration
      files: ["src/value.ts"]
      replaces: source
"#,
    );

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("manifest override should build");
    let entry = &built.document.entries[0];
    assert_eq!(entry.class, InventoryClass::Migration);
    assert_eq!(entry.by, "rule:manifest:inventory.classes[0]");
}

#[test]
fn manifest_class_override_errors_are_order_independent() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "README.md", "docs\n");
    write(root.path(), "src/value.ts", "export {};\n");
    let harmless = r#"    - class: doc
      files: ["README.md"]"#;

    for declarations in [
        format!(
            r#"    - class: migration
      files: ["src/value.ts"]
{harmless}"#
        ),
        format!(
            r#"{harmless}
    - class: migration
      files: ["src/value.ts"]"#
        ),
    ] {
        let manifest = manifest(&format!("  classes:\n{declarations}\n"));
        assert!(matches!(
            build_on_disk(
                root.path(),
                &snapshot(root.path()),
                &manifest,
                &BuildConfig::default()
            ),
            Err(InventoryError::MissingDefaultReplacement {
                path,
                default: InventoryClass::Source,
                ..
            }) if path == "src/value.ts"
        ));
    }

    for declarations in [
        format!(
            r#"    - class: migration
      files: ["src/value.ts"]
      replaces: test
{harmless}"#
        ),
        format!(
            r#"{harmless}
    - class: migration
      files: ["src/value.ts"]
      replaces: test"#
        ),
    ] {
        let manifest = manifest(&format!("  classes:\n{declarations}\n"));
        assert!(matches!(
            build_on_disk(
                root.path(),
                &snapshot(root.path()),
                &manifest,
                &BuildConfig::default()
            ),
            Err(InventoryError::WrongDefaultReplacement {
                path,
                expected: InventoryClass::Source,
                named: InventoryClass::Test,
                ..
            }) if path == "src/value.ts"
        ));
    }
}

#[test]
fn generated_provenance_distinguishes_same_producer_inputs() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/first.ts", "generated\n");
    write(root.path(), "generated/second.ts", "generated\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/first.ts"]
      producer: generate
      inputs: ["schema/first.yaml"]
    - files: ["generated/second.ts"]
      producer: generate
      inputs: ["schema/second.yaml"]
"#,
    );

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("generated declarations should build");
    let inputs = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.generated_by.as_ref())
            .and_then(|generated| generated.inputs.as_deref())
            .expect("generated provenance")
    };
    assert_eq!(inputs("generated/first.ts"), ["schema/first.yaml"]);
    assert_eq!(inputs("generated/second.ts"), ["schema/second.yaml"]);
}

#[test]
fn snapshot_listing_controls_paths_blobs_and_ignored_count() {
    let clean = tempdir().expect("clean checkout");
    let dirty = tempdir().expect("checkout with ignored files");
    write(clean.path(), "src/value.ts", "export {};\n");
    write(clean.path(), "src/z-other.ts", "export {};\n");
    write(dirty.path(), "src/value.ts", "export {};\n");
    write(dirty.path(), "src/z-other.ts", "export {};\n");
    write(dirty.path(), "target/cache.bin", "ignored\n");
    let listing = vec![
        snapshot_entry(
            "src/value.ts",
            InventoryClass::Unknown,
            Some("0123456789012345678901234567890123456789"),
        ),
        snapshot_entry(
            "src/z-other.ts",
            InventoryClass::Unknown,
            Some("0123456789012345678901234567890123456789012345678901234567890123"),
        ),
    ];

    let first = build_on_disk(
        clean.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("clean inventory");
    let second = build_on_disk(
        dirty.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("dirty inventory");
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.document.entries, second.document.entries);
    assert_eq!(
        first.document.entries[0].blob.as_deref(),
        Some("sha1:0123456789012345678901234567890123456789")
    );
    assert_eq!(
        first.document.entries[1].blob.as_deref(),
        Some("sha256:0123456789012345678901234567890123456789012345678901234567890123")
    );
    assert_eq!(first.document.summary.ignored_files, Some(0));

    let listing_with_ignored = [
        listing[0].clone(),
        snapshot_entry("target/cache.bin", InventoryClass::Ignored, None),
    ];
    let with_ignored = build_on_disk(
        dirty.path(),
        &listing_with_ignored,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("excluded path inventory");
    assert_eq!(with_ignored.document.summary.ignored_files, Some(1));
}

#[test]
fn malformed_cargo_manifest_with_lock_fails_closed() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "Cargo.toml", "[package\n");
    write(root.path(), "Cargo.lock", "version = 4\n");
    let error = discover_units(root.path()).expect_err("locked metadata failures must surface");
    assert!(
        matches!(error, InventoryError::InvalidDeclaration { reason }
        if reason.contains("cargo metadata failed") && reason.contains("Cargo.toml") && reason.contains("error"))
    );
}

#[test]
fn malformed_cargo_manifest_without_lock_keeps_conservative_units() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "Cargo.toml", "[package\n");
    let units = discover_units(root.path()).expect("no-lock fallback is conservative");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].root, ".");
    assert_eq!(units[0].configuration, "Cargo.toml");
    assert_eq!(units[0].by, "cargo-manifest");
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn discovers_monorepo_units_from_all_declared_sources() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "package.json",
        r#"{"private":true,"workspaces":["packages/*"]}"#,
    );
    write(root.path(), "tsconfig.json", "{}\n");
    write(
        root.path(),
        "packages/web/package.json",
        r#"{"name":"web"}"#,
    );
    write(root.path(), "packages/web/tsconfig.json", "{}\n");
    write(root.path(), "packages/web/src/index.ts", "export {};\n");
    write(
        root.path(),
        "pnpm-workspace.yaml",
        "packages:\n  - 'services/*'\n",
    );
    write(
        root.path(),
        "services/api/package.json",
        r#"{"name":"api"}"#,
    );
    write(root.path(), "services/api/src/index.ts", "export {};\n");
    write(
        root.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/tool\"]\nresolver = \"3\"\n",
    );
    write(
        root.path(),
        "crates/tool/Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(root.path(), "crates/tool/src/lib.rs", "pub fn tool() {}\n");

    let units = discover_units(root.path()).expect("units discovered");
    let roots: Vec<_> = units.iter().map(|unit| unit.root.as_str()).collect();
    assert!(roots.contains(&"."));
    assert!(roots.contains(&"packages/web"));
    assert!(roots.contains(&"services/api"));
    assert!(roots.contains(&"crates/tool"));

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    let unit_of = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.unit.as_deref())
    };
    assert_eq!(unit_of("packages/web/src/index.ts"), Some("packages/web"));
    assert_eq!(unit_of("services/api/src/index.ts"), Some("."));
    assert_eq!(unit_of("crates/tool/src/lib.rs"), Some("crates/tool"));
}

#[test]
fn project_references_define_units_and_tsconfig_precedes_deeper_package() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "tsconfig.json",
        r#"{"references":[{"path":"packages/app/tsconfig.build.json"}]}"#,
    );
    write(
        root.path(),
        "packages/app/tsconfig.build.json",
        r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#,
    );
    write(root.path(), "packages/app/src/index.ts", "export {};\n");
    write(
        root.path(),
        "packages/loose/package.json",
        r#"{"name":"loose"}"#,
    );
    write(root.path(), "packages/loose/src/index.ts", "export {};\n");

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("referenced units should build");
    let entry = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("fixture entry")
    };
    assert_eq!(
        entry("packages/app/src/index.ts").unit.as_deref(),
        Some("packages/app")
    );
    assert_eq!(
        entry("packages/loose/src/index.ts").unit.as_deref(),
        Some(".")
    );
    let app_alias = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/app")
        .expect("referenced unit alias row");
    assert_eq!(
        app_alias.alias_table.as_deref(),
        Some("packages/app/tsconfig.build.json")
    );
    assert_eq!(app_alias.by, "tsconfig-reference");
}

#[test]
fn alias_summary_has_one_explicit_row_per_unit() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "packages/exported/package.json",
        r#"{"name":"exported","exports":{".":"./src/index.js"}}"#,
    );
    write(
        root.path(),
        "packages/exported/src/index.js",
        "export {};\n",
    );
    write(
        root.path(),
        "packages/plain/package.json",
        r#"{"name":"plain"}"#,
    );
    write(root.path(), "packages/plain/src/index.js", "export {};\n");

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("package units should build");
    assert_eq!(built.units.len(), built.document.summary.unit_aliases.len());
    for unit in &built.units {
        assert_eq!(
            built
                .document
                .summary
                .unit_aliases
                .iter()
                .filter(|row| row.unit == unit.root)
                .count(),
            1,
            "each unit must have exactly one alias row"
        );
    }
    let exported = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/exported")
        .expect("exported unit row");
    assert_eq!(
        exported.alias_table.as_deref(),
        Some("packages/exported/package.json")
    );
    let plain = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/plain")
        .expect("plain unit row");
    assert_eq!(plain.alias_table, None);
}

#[test]
fn discovers_package_and_manifest_entrypoints() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "package.json",
        r#"{"bin":"./bin/cli.js","main":"./src/main.js","module":"./src/module.js","exports":{".":"./src/export.js"}}"#,
    );
    for path in [
        "bin/cli.js",
        "src/main.js",
        "src/module.js",
        "src/export.js",
        "src/job.ts",
    ] {
        write(root.path(), path, "export {};\n");
    }
    let manifest = WarrantManifest::parse(
        r#"
schema_version: warrant.manifest/1
integrations:
  lang-ts:
    enabled: true
entrypoints:
  - kind: workflow
    path: src/job.ts
    symbol: run
"#,
    )
    .expect("valid manifest");

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    let kinds = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("entrypoint path")
            .entrypoints
            .iter()
            .map(|entrypoint| entrypoint.kind.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds("bin/cli.js"), ["package-bin"]);
    assert_eq!(kinds("src/main.js"), ["package-main"]);
    assert_eq!(kinds("src/module.js"), ["package-main"]);
    assert_eq!(kinds("src/export.js"), ["package-export"]);
    assert_eq!(kinds("src/job.ts"), ["workflow"]);
}

#[test]
fn generated_verification_reports_drift_and_absence() {
    let root = tempdir().expect("temporary repository");
    let effects = tempdir().expect("producer execution sentinels");
    let reproducible_effect = effects.path().join("reproducible-ran");
    let unsafe_effect = effects.path().join("non-reproducible-ran");
    write(root.path(), "generated/value.txt", "stale\n");
    write(root.path(), "generated/clean.txt", "same\n");
    write(root.path(), "source.txt", "input\n");
    let clean_producer = serde_json::to_string(&format!(
        "mkdir -p generated && printf 'same\\n' > generated/clean.txt && printf ran > '{}'",
        reproducible_effect.display()
    ))
    .expect("quoted shell producer");
    let unsafe_producer =
        serde_json::to_string(&format!("printf ran > '{}'", unsafe_effect.display()))
            .expect("quoted shell producer");
    let manifest = manifest(&format!(
        r#"  generated:
    - files: ["generated/value.txt", "generated/missing.txt"]
      producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt"
      reproducible: true
    - files: ["generated/clean.txt"]
      producer: {clean_producer}
      reproducible: true
    - files: ["generated/unsafe.txt"]
      producer: {unsafe_producer}
      reproducible: false
"#,
    ));

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("inventory records declared missing output");
    let absent = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "generated/missing.txt")
        .expect("absent generated path has an inventory entry");
    assert_eq!(absent.class, InventoryClass::Generated);
    assert_eq!(absent.reason, "generated-absent");
    assert_eq!(
        absent
            .generated_by
            .as_ref()
            .expect("producer provenance")
            .producer,
        "mkdir -p generated && printf 'fresh\\n' > generated/value.txt"
    );

    let issues =
        verify_on_disk(root.path(), &snapshot(root.path()), &manifest).expect("producer runs");
    assert_eq!(
        fs::read_to_string(&reproducible_effect).expect("reproducible producer ran"),
        "ran"
    );
    assert!(
        !unsafe_effect.exists(),
        "non-reproducible producer must never execute"
    );
    assert_eq!(
        issues,
        [
            warrant_inventory::GeneratedIssue {
                code: GeneratedIssueCode::GeneratedAbsent,
                path: "generated/missing.txt".into(),
                producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt".into(),
            },
            warrant_inventory::GeneratedIssue {
                code: GeneratedIssueCode::GeneratedDrift,
                path: "generated/value.txt".into(),
                producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt".into(),
            },
        ]
    );
}

#[test]
fn glob_declared_absent_generated_scope_is_summarized_and_verified() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "source.txt", "input\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/**"]
      producer: "true"
      inputs: ["source.txt"]
      reproducible: true
"#,
    );

    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("absent generated scope should build");
    assert_eq!(built.document.summary.generated_absent.len(), 1);
    assert_eq!(
        built.document.summary.generated_absent[0].declaration,
        "generated/**"
    );
    assert_eq!(built.document.summary.generated_absent[0].producer, "true");
    assert!(
        built
            .document
            .entries
            .iter()
            .all(|entry| entry.path != "generated/**"),
        "a glob declaration is not a file path"
    );

    let issues =
        verify_on_disk(root.path(), &snapshot(root.path()), &manifest).expect("producer runs");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, GeneratedIssueCode::GeneratedAbsent);
    assert_eq!(issues[0].path, "generated/**");
}

#[test]
fn completeness_counts_equal_entries_and_digest_is_stable() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/owned.ts", "export {};\n");
    write(root.path(), "src/unowned.ts", "export {};\n");
    write(root.path(), "mystery.xyz", "?\n");
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "owned".into(),
            files: vec!["src/owned.ts".into()],
        }],
        ..BuildConfig::default()
    };
    let mut listing = snapshot(root.path());
    listing.push(snapshot_entry(
        "external",
        InventoryClass::Submodule,
        Some("sha1:123"),
    ));
    listing.push(snapshot_entry(
        "ignored/cache.bin",
        InventoryClass::Ignored,
        None,
    ));

    let first =
        build_on_disk(root.path(), &listing, &empty_manifest(), &config).expect("first inventory");
    let second =
        build_on_disk(root.path(), &listing, &empty_manifest(), &config).expect("second inventory");
    let counted: u64 = first.document.summary.by_class.values().sum();
    assert_eq!(counted, first.document.entries.len() as u64);
    // The ignored entry is listed and classed, but it is outside the snapshot.
    assert_eq!(first.document.summary.by_class["ignored"], 1);
    assert_eq!(first.document.summary.files, counted - 1);
    assert_eq!(first.document.summary.unowned_source, ["src/unowned.ts"]);
    assert_eq!(first.document.summary.unknown, ["mystery.xyz"]);
    assert_eq!(first.document.summary.submodules[0].path, "external");
    assert_eq!(first.document.summary.ignored_files, Some(1));
    assert_eq!(first.digest, second.digest);
}

#[test]
fn inventory_digest_changes_with_captured_file_content() {
    let first_root = tempdir().expect("first repository");
    let second_root = tempdir().expect("second repository");
    write(
        first_root.path(),
        "src/value.ts",
        "export const value = 1;\n",
    );
    write(
        second_root.path(),
        "src/value.ts",
        "export const value = 2;\n",
    );
    let capture = |root: &Path| {
        build_on_disk(
            root,
            &snapshot(root),
            &empty_manifest(),
            &BuildConfig::default(),
        )
        .expect("inventory builds")
    };
    let first = capture(first_root.path());
    let second = capture(second_root.path());
    assert_ne!(
        first.document.entries[0].blob,
        second.document.entries[0].blob
    );
    assert!(first.digest.starts_with("sha256:"));
    assert!(second.digest.starts_with("sha256:"));
    assert_ne!(
        first.digest, second.digest,
        "content changes must change the inventory identity"
    );
}

#[cfg(unix)]
#[test]
fn unread_snapshot_entry_preserves_read_failure_in_completeness() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/unread.ts", "export {};\n");
    let path = root.path().join("src/unread.ts");
    let permissions = fs::metadata(&path).expect("fixture metadata").permissions();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("make fixture unreadable");
    let read_result = fs::read(&path);
    let listing = snapshot(root.path());
    let result = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    );
    fs::set_permissions(&path, permissions).expect("restore fixture permissions");
    let error = read_result
        .expect_err("this test needs an unprivileged user to exercise a real read failure");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    let built = result.expect("unread entries stay in the inventory");
    assert_eq!(built.document.entries.len(), 1);
    assert_eq!(built.document.entries[0].class, InventoryClass::Unread);
    assert_eq!(
        built.document.entries[0].unread.as_deref(),
        Some(error.to_string().as_str())
    );
    assert_eq!(built.document.summary.unread.len(), 1);
    assert_eq!(built.document.summary.unread[0].path, "src/unread.ts");
    assert_eq!(built.document.summary.unread[0].reason, error.to_string());
}

/// Spec 4.3: the ignored count is known only for a worktree capture; an object snapshot
/// reports it as unknown (null), which is a different claim from zero.
#[test]
fn ignored_count_is_unknown_outside_the_worktree() {
    let temp = tempdir().expect("temp");
    let root = temp.path();
    write(root, "README.md", "readme\n");
    let listing = vec![snapshot_entry(
        "README.md",
        InventoryClass::Unknown,
        Some("0123456789012345678901234567890123456789"),
    )];
    let manifest =
        WarrantManifest::parse("schema_version: warrant.manifest/1\n").expect("manifest");
    let config = BuildConfig::default();
    for (kind, expected) in [
        (SnapshotKind::Worktree, Some(0)),
        (SnapshotKind::Index, None),
        (SnapshotKind::Commit, None),
        (SnapshotKind::Tree, None),
    ] {
        let snapshot = SnapshotManifest {
            kind: kind.clone(),
            ..worktree_manifest()
        };
        let reader = |path: &str| {
            fs::read(root.join(path)).map_err(|error| ReadError {
                code: "io".into(),
                reason: error.to_string(),
            })
        };
        let built = build(
            root,
            CapturedSnapshot {
                manifest: &snapshot,
                entries: &listing,
                read: &reader,
                untracked: &BTreeSet::new(),
            },
            &manifest,
            &config,
        )
        .expect("inventory");
        assert_eq!(built.document.summary.ignored_files, expected, "{kind:?}");
        assert_eq!(
            built.document.snapshot.as_ref(),
            Some(&snapshot),
            "{kind:?}"
        );
    }
}

/// A bare snapshot blob id is prefixed by its object format; a length that names no
/// format is an error, not a guess.
#[test]
fn snapshot_blob_of_unrecognized_length_is_rejected() {
    let root = tempdir().expect("temporary repository");
    let listing = [snapshot_entry(
        "notes.xyz",
        InventoryClass::Unknown,
        Some("abc123"),
    )];
    let error = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect_err("a six-character blob id has no object format");
    assert_eq!(error.code(), "invalid-declaration");
    assert_eq!(
        error.to_string(),
        "snapshot blob `abc123` has no recognized object format"
    );
}

/// A blob id of SHA-1 length that is not hexadecimal is rejected, not prefixed.
#[test]
fn snapshot_blob_that_is_not_hexadecimal_is_rejected() {
    let root = tempdir().expect("temporary repository");
    let blob = "z".repeat(40);
    let listing = [snapshot_entry(
        "notes.xyz",
        InventoryClass::Unknown,
        Some(&blob),
    )];
    let error = build_on_disk(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect_err("a non-hex blob id is not an object id");
    assert_eq!(error.code(), "invalid-declaration");
    assert_eq!(
        error.to_string(),
        format!("snapshot blob `{blob}` is not a hexadecimal object id")
    );
}

/// An entry's producer provenance keeps the manifest's distinction: no `inputs` is
/// undeclared (null) and `inputs: []` is a producer declared to read nothing.
#[test]
fn generated_provenance_keeps_undeclared_and_empty_inputs_apart() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "gen/undeclared.ts", "export {};\n");
    write(root.path(), "gen/none.ts", "export {};\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["gen/undeclared.ts"]
      producer: first
    - files: ["gen/none.ts"]
      producer: second
      inputs: []
"#,
    );
    let built = build_on_disk(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("generated declarations should build");
    let inputs = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.generated_by.clone())
            .expect("generated provenance")
            .inputs
    };
    assert_eq!(inputs("gen/undeclared.ts"), None);
    assert_eq!(inputs("gen/none.ts"), Some(Vec::new()));
}

/// B25: verification's absences join discovery's rows unless a discovery row for the
/// same producer already covers the path; drift is recorded as the verified result.
#[test]
fn verification_absences_join_discovery_rows_without_duplicates() {
    let absent = |declaration: &str, producer: &str| GeneratedAbsent {
        declaration: declaration.into(),
        producer: producer.into(),
    };
    let issue = |code, path: &str, producer: &str| GeneratedIssue {
        code,
        path: path.into(),
        producer: producer.into(),
    };
    let mut summary = InventorySummary {
        generated_absent: vec![absent("out/*.js", "build"), absent("lit.ts", "lit")],
        ..InventorySummary::default()
    };
    record_verification(
        &mut summary,
        vec![
            issue(GeneratedIssueCode::GeneratedAbsent, "gen/b.ts", "gen"),
            issue(GeneratedIssueCode::GeneratedAbsent, "lit.ts", "lit"),
            issue(GeneratedIssueCode::GeneratedAbsent, "out/a.js", "build"),
            issue(GeneratedIssueCode::GeneratedAbsent, "out/c.js", "other"),
            issue(GeneratedIssueCode::GeneratedDrift, "gen/a.ts", "gen"),
        ],
    )
    .expect("record verification");
    assert_eq!(
        summary.generated_absent,
        [
            absent("out/*.js", "build"),
            absent("lit.ts", "lit"),
            absent("gen/b.ts", "gen"),
            absent("out/c.js", "other"),
        ]
    );
    assert_eq!(
        summary.generated_drift,
        Some(vec![GeneratedDrift {
            path: "gen/a.ts".into(),
            producer: "gen".into(),
        }])
    );
}
