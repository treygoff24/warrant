//! Inventory invariants driven through `warrant_snapshot::capture` over real temporary
//! repositories, so class, ownership and discovery see the entries the product captures
//! (ignored, unread and symlinked paths included) rather than a filesystem walk.

use std::{cell::RefCell, collections::BTreeMap, fs, path::Path, process::Command, sync::OnceLock};

use warrant_core::{
    manifest::WarrantManifest,
    nouns::{InventoryClass, InventoryEntry, SnapshotKind},
};
use warrant_inventory::{BuildConfig, BuiltInventory, CapturedSnapshot, InventoryError, ReadError};
use warrant_snapshot::SnapshotError;

const CHILD: &str = "WARRANT_TEST_NEUTRAL_GIT_CHILD";

/// Git in these tests sees no system or global configuration and no inherited
/// repository redirection.
fn neutralize(command: &mut Command) {
    static EMPTY: OnceLock<tempfile::NamedTempFile> = OnceLock::new();
    let empty = EMPTY.get_or_init(|| tempfile::NamedTempFile::new().expect("empty Git config"));
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty.path());
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
    ] {
        command.env_remove(name);
    }
}

/// In-process capture shells out to Git with this process's environment, so each test
/// body runs in a child copy of this binary whose environment is neutral.
fn in_neutral_git_child(test: &str, body: impl FnOnce()) {
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let mut command = Command::new(std::env::current_exe().expect("test binary"));
    command
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1");
    neutralize(&mut command);
    let output = command.output().expect("run neutral Git child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{test} child failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "{test} child ran no test:\n{stdout}"
    );
}

struct Repository(tempfile::TempDir);

impl Repository {
    fn new() -> Self {
        let repository = Self(tempfile::tempdir().expect("temporary repository"));
        repository.git(&["init", "-q"]);
        repository
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn write(&self, path: &str, contents: &str) {
        let target = self.root().join(path);
        fs::create_dir_all(target.parent().expect("file parent")).expect("create parent");
        fs::write(target, contents).expect("write fixture file");
    }

    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        command.arg("-C").arg(self.root()).args(args);
        neutralize(&mut command);
        let output = command.output().expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8 Git output")
    }

    /// Stage everything Git does not ignore and commit it under an explicit identity.
    fn commit_all(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&[
            "-c",
            "user.name=Warrant Test",
            "-c",
            "user.email=warrant-test@example.invalid",
            "commit",
            "-q",
            "-m",
            message,
        ]);
    }
}

fn manifest(inventory: &str) -> WarrantManifest {
    WarrantManifest::parse(&format!(
        "schema_version: warrant.manifest/1\nintegrations:\n  lang-ts:\n    enabled: true\n  lang-rust:\n    enabled: true\n{inventory}"
    ))
    .expect("valid fixture manifest")
}

/// One captured inventory and every path discovery asked the snapshot for.
struct Captured {
    built: BuiltInventory,
    requested: Vec<String>,
}

impl Captured {
    fn entry(&self, path: &str) -> &InventoryEntry {
        self.built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing inventory entry {path}"))
    }
}

/// Capture `kind` and build the inventory from the capture's own entries and bytes, the
/// way `warrant inventory` does.
fn capture(
    repository: &Repository,
    kind: SnapshotKind,
    revision: Option<&str>,
    manifest: &WarrantManifest,
    config: &BuildConfig,
) -> Result<Captured, SnapshotError> {
    try_capture(repository, kind, revision, manifest, config)
        .map(|built| built.unwrap_or_else(|error| panic!("inventory failed: {error}")))
}

/// As `capture`, returning the inventory error instead of failing the test.
fn try_capture(
    repository: &Repository,
    kind: SnapshotKind,
    revision: Option<&str>,
    manifest: &WarrantManifest,
    config: &BuildConfig,
) -> Result<Result<Captured, InventoryError>, SnapshotError> {
    let root = repository.root();
    warrant_snapshot::capture(root, kind, revision, &manifest.snapshot, |snapshot| {
        let requested = RefCell::new(Vec::new());
        let read = |path: &str| {
            requested.borrow_mut().push(path.to_owned());
            snapshot.read(path).map_err(|error| ReadError {
                code: error.document.code,
                reason: error.document.reason,
            })
        };
        let resolve = |path: &str| {
            snapshot.resolve(path).map_err(|error| ReadError {
                code: error.document.code,
                reason: error.document.reason,
            })
        };
        let built = warrant_inventory::build(
            root,
            CapturedSnapshot {
                manifest: snapshot.manifest(),
                entries: snapshot.entries(),
                read: &read,
                resolve: &resolve,
                untracked: snapshot.untracked(),
            },
            manifest,
            config,
        );
        Ok(built.map(|built| Captured {
            built,
            requested: requested.into_inner(),
        }))
    })
    .map(|(_, captured)| captured)
}

fn worktree(repository: &Repository, manifest: &WarrantManifest) -> Captured {
    capture(
        repository,
        SnapshotKind::Worktree,
        None,
        manifest,
        &BuildConfig::default(),
    )
    .unwrap_or_else(|error| panic!("worktree capture: {error}"))
}

/// W0.5: a first-party file outside every module selector is listed under
/// `unowned_source`, never silently classified.
#[test]
fn first_party_source_outside_module_is_unowned() {
    in_neutral_git_child("first_party_source_outside_module_is_unowned", || {
        let repository = Repository::new();
        repository.write("outside.ts", "export {};\n");
        repository.write("tools/run.mjs", "export {};\n");
        repository.commit_all("outside");

        let captured = worktree(
            &repository,
            &manifest(
                "inventory:\n  classes:\n    - class: source\n      files: [\"tools/*.mjs\"]\n",
            ),
        );
        let outside = captured.entry("outside.ts");
        assert_eq!(outside.class, InventoryClass::Source);
        assert_eq!(outside.module, None);
        assert_eq!(outside.unit.as_deref(), Some("."));
        // `by` names the rule that classified the file, not the unit fallback.
        assert_eq!(outside.by, "default:source");
        let declared = captured.entry("tools/run.mjs");
        assert_eq!(declared.class, InventoryClass::Source);
        assert_eq!(declared.unit.as_deref(), Some("."));
        assert_eq!(declared.by, "rule:manifest:inventory.classes[0]");
        let summary = &captured.built.document.summary;
        assert_eq!(summary.unowned_source, ["outside.ts", "tools/run.mjs"]);
        assert_eq!(summary.unit_aliases.len(), 1);
        assert_eq!(summary.unit_aliases[0].unit, ".");
        assert_eq!(summary.unit_aliases[0].alias_table, None);
        assert_eq!(summary.unit_aliases[0].by, "implicit-root-fallback");
    });
}

/// B1: a package.json that is a symlink leaving the tree is unread, so it declares no
/// unit, workspace or entrypoint, and discovery never asks the snapshot for its bytes.
#[test]
fn external_symlink_package_json_does_not_supply_units_or_entrypoints() {
    in_neutral_git_child(
        "external_symlink_package_json_does_not_supply_units_or_entrypoints",
        || {
            let outside = tempfile::tempdir().expect("outside directory");
            fs::write(
                outside.path().join("package.json"),
                r#"{"main":"./src/index.ts","workspaces":["packages/*"]}"#,
            )
            .expect("write outside package.json");
            let repository = Repository::new();
            repository.write("src/index.ts", "export const index = 1;\n");
            repository.write("packages/a/package.json", "{}");
            repository.write("packages/a/src/a.ts", "export const a = 1;\n");
            std::os::unix::fs::symlink(
                outside.path().join("package.json"),
                repository.root().join("package.json"),
            )
            .expect("external symlink");
            repository.commit_all("external symlink");

            let captured = worktree(&repository, &manifest(""));
            assert!(
                !captured.requested.iter().any(|path| path == "package.json"),
                "discovery read the unread entry: {:?}",
                captured.requested
            );
            assert_eq!(
                captured.entry("package.json").unread.as_deref(),
                Some("external-symlink")
            );
            assert!(
                captured
                    .built
                    .document
                    .summary
                    .unread
                    .iter()
                    .any(|item| item.path == "package.json" && item.reason == "external-symlink")
            );
            assert!(captured.entry("src/index.ts").entrypoints.is_empty());
            let units: BTreeMap<_, _> = captured
                .built
                .units
                .iter()
                .map(|unit| {
                    (
                        unit.root.as_str(),
                        (unit.configuration.as_str(), unit.by.as_str()),
                    )
                })
                .collect();
            assert!(
                !units
                    .values()
                    .any(|(configuration, _)| *configuration == "package.json"),
                "{units:?}"
            );
            // The workspace glob lived only in the external file; the package keeps its own unit.
            assert_eq!(
                units.get("packages/a"),
                Some(&("packages/a/package.json", "package-json"))
            );
        },
    );
}

/// B1: an oversize tsconfig is unread in the snapshot and supplies no unit or alias table.
#[test]
fn oversize_tsconfig_is_unread_and_supplies_no_unit_or_alias_table() {
    in_neutral_git_child(
        "oversize_tsconfig_is_unread_and_supplies_no_unit_or_alias_table",
        || {
            let repository = Repository::new();
            let tsconfig = r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#;
            repository.write("tsconfig.json", tsconfig);
            repository.write("src/index.ts", "export const index = 1;\n");
            repository.commit_all("oversize tsconfig");
            let limit = 40;
            assert!(tsconfig.len() > limit, "the fixture must exceed the limit");
            let manifest = manifest(&format!("snapshot:\n  max_file_bytes: {limit}\n"));

            let captured = worktree(&repository, &manifest);
            assert!(
                !captured
                    .requested
                    .iter()
                    .any(|path| path == "tsconfig.json"),
                "{:?}",
                captured.requested
            );
            assert_eq!(
                captured.entry("tsconfig.json").class,
                InventoryClass::Unread
            );
            assert!(
                captured
                    .built
                    .document
                    .summary
                    .unread
                    .iter()
                    .any(|item| item.path == "tsconfig.json" && item.reason == "oversize")
            );
            assert!(
                !captured
                    .built
                    .units
                    .iter()
                    .any(|unit| unit.configuration == "tsconfig.json")
            );
            assert_eq!(captured.entry("src/index.ts").unit.as_deref(), Some("."));
            assert!(
                captured
                    .built
                    .document
                    .summary
                    .unit_aliases
                    .iter()
                    .all(|row| row.alias_table.is_none()),
                "{:?}",
                captured.built.document.summary.unit_aliases
            );
        },
    );
}

/// B1: discovery reads the captured object's bytes, not the file now on disk.
#[test]
fn discovery_follows_captured_bytes_not_the_live_file() {
    in_neutral_git_child("discovery_follows_captured_bytes_not_the_live_file", || {
        let repository = Repository::new();
        repository.write("package.json", r#"{"main":"./captured.ts"}"#);
        repository.write(
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@/*":["./*"]}}}"#,
        );
        repository.write("live.ts", "export const live = 1;\n");
        repository.write("captured.ts", "export const captured = 1;\n");
        repository.commit_all("captured configuration");
        // The worktree now disagrees with the commit being inventoried.
        repository.write("package.json", r#"{"main":"./live.ts"}"#);
        repository.write("tsconfig.json", "{}");

        let captured = capture(
            &repository,
            SnapshotKind::Commit,
            Some("HEAD"),
            &manifest(""),
            &BuildConfig::default(),
        )
        .unwrap_or_else(|error| panic!("commit capture: {error}"));
        assert!(
            captured.requested.iter().any(|path| path == "package.json"),
            "{:?}",
            captured.requested
        );
        assert_eq!(captured.entry("captured.ts").entrypoints.len(), 1);
        assert_eq!(
            captured.entry("captured.ts").entrypoints[0].kind,
            "package-main"
        );
        assert!(captured.entry("live.ts").entrypoints.is_empty());
        let root_alias = captured
            .built
            .document
            .summary
            .unit_aliases
            .iter()
            .find(|row| row.unit == ".")
            .expect("root unit alias row");
        assert_eq!(root_alias.alias_table.as_deref(), Some("tsconfig.json"));
    });
}

const CARGO_WORKSPACE: &str = "[workspace]\nmembers = [\"crates/a\"]\nresolver = \"2\"\n";
const CARGO_MEMBER: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const CARGO_LOCK: &str = "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"a\"\nversion = \"0.1.0\"\n";

/// B24: `cargo metadata` runs over the captured Cargo inputs, so a malformed live
/// manifest does not fail a commit whose captured manifests are valid.
#[test]
fn cargo_discovery_follows_captured_manifests_not_the_live_files() {
    in_neutral_git_child(
        "cargo_discovery_follows_captured_manifests_not_the_live_files",
        || {
            let repository = Repository::new();
            repository.write("Cargo.toml", CARGO_WORKSPACE);
            repository.write("Cargo.lock", CARGO_LOCK);
            repository.write("crates/a/Cargo.toml", CARGO_MEMBER);
            repository.write("crates/a/src/lib.rs", "pub fn a() {}\n");
            repository.commit_all("captured cargo workspace");
            // The worktree now disagrees with the commit being inventoried.
            repository.write("Cargo.toml", "[workspace\n");
            repository.write("crates/a/Cargo.toml", "not toml [\n");

            let captured = capture(
                &repository,
                SnapshotKind::Commit,
                Some("HEAD"),
                &manifest(""),
                &BuildConfig::default(),
            )
            .unwrap_or_else(|error| panic!("commit capture: {error}"));
            for path in ["Cargo.toml", "Cargo.lock", "crates/a/Cargo.toml"] {
                assert!(
                    captured.requested.iter().any(|item| item == path),
                    "{path} not read through the snapshot: {:?}",
                    captured.requested
                );
            }
            assert_eq!(
                captured.entry("crates/a/src/lib.rs").unit.as_deref(),
                Some("crates/a")
            );
        },
    );
}

/// B24: an oversize Cargo manifest is refused with the snapshot's own code, never
/// skipped while Cargo reads the live file.
#[test]
fn oversize_cargo_manifest_is_refused_with_the_snapshot_code() {
    in_neutral_git_child(
        "oversize_cargo_manifest_is_refused_with_the_snapshot_code",
        || {
            let repository = Repository::new();
            repository.write("Cargo.toml", CARGO_MEMBER);
            repository.write("src/lib.rs", "");
            repository.commit_all("oversize cargo manifest");
            let limit = 40;
            assert!(
                CARGO_MEMBER.len() > limit,
                "the fixture must exceed the limit"
            );
            let manifest = manifest(&format!("snapshot:\n  max_file_bytes: {limit}\n"));

            let result = try_capture(
                &repository,
                SnapshotKind::Worktree,
                None,
                &manifest,
                &BuildConfig::default(),
            )
            .unwrap_or_else(|error| panic!("worktree capture: {error}"));
            match result {
                Err(InventoryError::Read { path, code, .. }) => {
                    assert_eq!((path.as_str(), code.as_str()), ("Cargo.toml", "oversize"));
                }
                Err(error) => panic!("unexpected inventory error: {error}"),
                Ok(captured) => panic!(
                    "oversize Cargo.toml was not refused; units {:?}",
                    captured.built.units
                ),
            }
        },
    );
}

/// B1: a read the snapshot refuses keeps the snapshot's code, so a worktree that keeps
/// changing under discovery ends in the capture's own `snapshot-unstable`.
#[test]
fn refused_snapshot_read_keeps_the_snapshot_code() {
    in_neutral_git_child("refused_snapshot_read_keeps_the_snapshot_code", || {
        let repository = Repository::new();
        repository.write("package.json", "{}");
        repository.commit_all("package");
        let root = repository.root();
        let manifest = manifest("");
        let codes = RefCell::new(Vec::new());
        let error = warrant_snapshot::capture(
            root,
            SnapshotKind::Worktree,
            None,
            &manifest.snapshot,
            |snapshot| {
                // Another writer changes the file after capture and before discovery.
                let attempt = codes.borrow().len();
                fs::write(
                    root.join("package.json"),
                    format!(r#"{{"name":"changed-{attempt}"}}"#),
                )
                .expect("change package.json");
                let read = |path: &str| {
                    snapshot.read(path).map_err(|error| ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
                };
                let resolve = |path: &str| Ok(path.to_owned());
                let error = warrant_inventory::build(
                    root,
                    CapturedSnapshot {
                        manifest: snapshot.manifest(),
                        entries: snapshot.entries(),
                        read: &read,
                        resolve: &resolve,
                        untracked: &std::collections::BTreeSet::new(),
                    },
                    &manifest,
                    &BuildConfig::default(),
                )
                .expect_err("a refused read is not silently skipped");
                let InventoryError::Read { path, code, reason } = error else {
                    panic!("expected a read error, got {error:?}");
                };
                assert_eq!(path, "package.json");
                codes.borrow_mut().push(code.clone());
                Err::<(), _>(SnapshotError {
                    document: warrant_core::nouns::ErrorDocument {
                        schema_version: "warrant.error/1".into(),
                        code,
                        reason,
                        locations: Vec::new(),
                        next_diagnostic: None,
                    },
                })
            },
        )
        .expect_err("a worktree that keeps changing is unstable");
        assert_eq!(codes.into_inner(), ["snapshot-changed", "snapshot-changed"]);
        assert_eq!(error.document.code, "snapshot-unstable");
    });
}

/// B4, spec 5.3: "Generated files that are gitignored and therefore absent from the
/// snapshot are recorded as `generated-absent` with their producer".
#[test]
fn generated_declarations_matched_only_by_ignored_files_are_absent() {
    in_neutral_git_child(
        "generated_declarations_matched_only_by_ignored_files_are_absent",
        || {
            let repository = Repository::new();
            repository.write(".gitignore", "gen/\nout/\nlib/extra.ts\n");
            repository.write("src/index.ts", "export {};\n");
            repository.write("lib/tracked.ts", "export {};\n");
            repository.commit_all("sources");
            // A copy of each ignored output is on disk, as it is after a local build.
            let outputs = ["gen/client.ts", "out/a.js", "out/b.js", "lib/extra.ts"];
            for path in outputs {
                repository.write(path, "generated\n");
            }
            let manifest = manifest(
                r#"inventory:
  generated:
    - files: ["gen/client.ts"]
      producer: "make client"
    - files: ["out/*.js"]
      producer: "make out"
    - files: ["lib/*.ts"]
      producer: "make lib"
"#,
            );

            let captured = worktree(&repository, &manifest);
            let absent: Vec<_> = captured
                .built
                .document
                .summary
                .generated_absent
                .iter()
                .map(|item| (item.declaration.as_str(), item.producer.as_str()))
                .collect();
            // lib/*.ts also matches the captured lib/tracked.ts, so it is present.
            assert_eq!(
                absent,
                [("gen/client.ts", "make client"), ("out/*.js", "make out")]
            );
            // The ignored entries are the capture's own; none is duplicated or reclassified.
            for path in outputs {
                let matching: Vec<_> = captured
                    .built
                    .document
                    .entries
                    .iter()
                    .filter(|entry| entry.path == path)
                    .collect();
                assert_eq!(matching.len(), 1, "{path}");
                assert_eq!(matching[0].class, InventoryClass::Ignored, "{path}");
                assert_eq!(matching[0].by, "git-ignore", "{path}");
                assert_eq!(matching[0].generated_by, None, "{path}");
            }
            assert_eq!(
                captured.entry("lib/tracked.ts").class,
                InventoryClass::Generated
            );
        },
    );
}

/// B8, spec 5.2: "A file whose extension belongs to an enabled integration and that no
/// rule classifies is `source` with `module: null`". The build-output directory names
/// apply only to files Git does not track.
#[test]
fn build_output_default_never_reclassifies_a_tracked_file() {
    in_neutral_git_child(
        "build_output_default_never_reclassifies_a_tracked_file",
        || {
            let repository = Repository::new();
            repository.write("src/build/index.ts", "export const build = 1;\n");
            repository.write("scripts/build/x.ts", "export const x = 1;\n");
            repository.commit_all("tracked sources under build directories");
            // Untracked, non-ignored output a local build left behind.
            repository.write("dist/bundle.js", "bundle();\n");
            repository.write("node_modules/pkg/index.js", "module.exports = 1;\n");
            assert_eq!(
                repository.git(&["ls-files", "--others", "--exclude-standard"]),
                "dist/bundle.js\nnode_modules/pkg/index.js\n"
            );

            let captured = worktree(&repository, &manifest(""));
            for path in ["src/build/index.ts", "scripts/build/x.ts"] {
                let entry = captured.entry(path);
                assert_eq!(entry.class, InventoryClass::Source, "{path}");
                assert_eq!(entry.module, None, "{path}");
            }
            assert_eq!(
                captured.built.document.summary.unowned_source,
                ["scripts/build/x.ts", "src/build/index.ts"]
            );
            for path in ["dist/bundle.js", "node_modules/pkg/index.js"] {
                assert_eq!(
                    captured.entry(path).class,
                    InventoryClass::BuildOutput,
                    "{path}"
                );
            }

            // A commit has no untracked files: every entry is classified by extension.
            repository.commit_all("commit the output too");
            let committed = capture(
                &repository,
                SnapshotKind::Commit,
                Some("HEAD"),
                &manifest(""),
                &BuildConfig::default(),
            )
            .unwrap_or_else(|error| panic!("commit capture: {error}"));
            assert_eq!(
                committed.entry("dist/bundle.js").class,
                InventoryClass::Source
            );
        },
    );
}

/// A worktree inventory's error, for a repository the inventory must refuse.
fn worktree_error(repository: &Repository) -> InventoryError {
    match try_capture(
        repository,
        SnapshotKind::Worktree,
        None,
        &manifest(""),
        &BuildConfig::default(),
    ) {
        Ok(Err(error)) => error,
        Ok(Ok(_)) => panic!("the inventory was built"),
        Err(error) => panic!("worktree capture: {error}"),
    }
}

/// B9: a nested repository under an ignored directory is outside the snapshot, as this
/// repository's own `/.worktrees/<name>/.git` checkouts are.
#[test]
fn nested_repository_under_an_ignored_directory_is_not_looked_at() {
    in_neutral_git_child(
        "nested_repository_under_an_ignored_directory_is_not_looked_at",
        || {
            let repository = Repository::new();
            repository.write(".gitignore", ".worktrees/\nnode_modules/\n");
            repository.write("src/index.ts", "export {};\n");
            repository.commit_all("sources");
            for nested in [".worktrees/feature", "node_modules/pkg"] {
                repository.write(&format!("{nested}/src/copy.ts"), "export {};\n");
                repository.git(&["-C", nested, "init", "-q"]);
            }
            assert_eq!(
                repository.git(&["ls-files", "--others", "--ignored", "--exclude-standard"]),
                ".worktrees/feature/\nnode_modules/pkg/\n"
            );

            let captured = worktree(&repository, &manifest(""));
            assert_eq!(captured.entry("src/index.ts").class, InventoryClass::Source);
        },
    );
}

/// B9, spec 5.6: "A nested git repository that is not a submodule is an error in v1
/// (`nested-repository`), because its files have two identities."
#[test]
fn nested_repository_beside_tracked_files_is_rejected() {
    in_neutral_git_child("nested_repository_beside_tracked_files_is_rejected", || {
        // Untracked: Git lists the nested repository as one directory and never descends.
        let repository = Repository::new();
        repository.write("vendor/a.ts", "export {};\n");
        repository.commit_all("tracked vendor file");
        repository.write("vendor/nested/src/b.ts", "export {};\n");
        repository.git(&["-C", "vendor/nested", "init", "-q"]);
        assert_eq!(
            repository.git(&["ls-files", "--others", "--exclude-standard"]),
            "vendor/nested/\n"
        );
        let error = worktree_error(&repository);
        assert!(
            matches!(&error, InventoryError::NestedRepository { path } if path == "vendor/nested"),
            "{error:?}"
        );

        // Tracked by the outer repository and inside the nested one: two identities.
        let repository = Repository::new();
        repository.write("vendor/nested/src/b.ts", "export {};\n");
        repository.commit_all("tracked nested file");
        repository.git(&["-C", "vendor/nested", "init", "-q"]);
        let error = worktree_error(&repository);
        assert!(
            matches!(&error, InventoryError::NestedRepository { path } if path == "vendor/nested"),
            "{error:?}"
        );
    });
}

/// `summary.files` counts what the snapshot holds: tracked and untracked files, not the
/// ignored entries the worktree lists or a declared generated file that is absent.
#[test]
fn summary_files_counts_present_non_ignored_entries() {
    in_neutral_git_child("summary_files_counts_present_non_ignored_entries", || {
        let repository = Repository::new();
        repository.write(".gitignore", "build/\n");
        repository.write("src/index.ts", "export {};\n");
        repository.commit_all("sources");
        repository.write("src/new.ts", "export {};\n");
        repository.write("build/out.js", "built\n");
        let manifest = manifest(
            "inventory:\n  generated:\n    - files: [\"gen/missing.ts\"]\n      producer: make\n",
        );

        let captured = worktree(&repository, &manifest);
        let document = &captured.built.document;
        let listed: Vec<_> = document
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.path.as_str(),
                    entry.class.as_str(),
                    entry.reason.as_str(),
                )
            })
            .collect();
        // Precondition: the ignored and the absent entries are listed, so the count
        // below is a real exclusion and not an empty set.
        assert!(
            listed.iter().any(|(_, class, _)| *class == "ignored")
                && listed.contains(&("gen/missing.ts", "generated", "generated-absent")),
            "{listed:?}"
        );
        assert_eq!(document.summary.files, 3, "{listed:?}");
    });
}

/// Each discovered unit's root with its configuration and basis.
fn units_of(captured: &Captured) -> BTreeMap<String, (String, String)> {
    captured
        .built
        .units
        .iter()
        .map(|unit| {
            (
                unit.root.clone(),
                (unit.configuration.clone(), unit.by.clone()),
            )
        })
        .collect()
}

/// Each entry's entrypoint kinds, for entries that have any.
fn entrypoints_of(captured: &Captured) -> BTreeMap<String, Vec<String>> {
    captured
        .built
        .document
        .entries
        .iter()
        .filter(|entry| !entry.entrypoints.is_empty())
        .map(|entry| {
            (
                entry.path.clone(),
                entry
                    .entrypoints
                    .iter()
                    .map(|entrypoint| entrypoint.kind.clone())
                    .collect(),
            )
        })
        .collect()
}

/// Write each file, then make each `(link, target)` a symlink when `linked`, or a regular
/// copy of the target's contents otherwise, and commit.
fn configuration_repository(
    files: &[(&str, &str)],
    links: &[(&str, &str)],
    linked: bool,
) -> Repository {
    let repository = Repository::new();
    for (path, contents) in files {
        repository.write(path, contents);
    }
    for (link, target) in links {
        let path = repository.root().join(link);
        if linked {
            std::os::unix::fs::symlink(target, &path).expect("internal symlink");
        } else {
            let resolved = path.parent().expect("link parent").join(target);
            fs::copy(resolved, &path).expect("copy link target");
        }
    }
    repository.commit_all("configuration");
    repository
}

/// B28: a `package.json` that is a symlink to a file inside the tree declares the same
/// units, workspaces and entrypoints as the regular file, rooted at the link's own path.
#[test]
fn internal_symlink_package_json_supplies_the_same_units_as_a_regular_file() {
    in_neutral_git_child(
        "internal_symlink_package_json_supplies_the_same_units_as_a_regular_file",
        || {
            let files = [
                (
                    "config/root-package.json",
                    r#"{"main":"./src/index.ts","workspaces":["packages/*"]}"#,
                ),
                ("src/index.ts", "export const index = 1;\n"),
                ("packages/a/package.json", "{}"),
                ("packages/a/src/a.ts", "export const a = 1;\n"),
            ];
            let links = [("package.json", "config/root-package.json")];
            let regular = worktree(
                &configuration_repository(&files, &links, false),
                &manifest(""),
            );
            let linked = worktree(
                &configuration_repository(&files, &links, true),
                &manifest(""),
            );
            // Precondition: the regular file's declarations are observable.
            let units = units_of(&regular);
            assert_eq!(
                units.get("."),
                Some(&("package.json".into(), "package-json".into()))
            );
            assert_eq!(
                units.get("packages/a"),
                Some(&("packages/a/package.json".into(), "package-workspace".into()))
            );
            assert_eq!(
                entrypoints_of(&regular).get("src/index.ts"),
                Some(&vec!["package-main".to_owned()])
            );
            assert_eq!(units_of(&linked), units);
            assert_eq!(entrypoints_of(&linked), entrypoints_of(&regular));
        },
    );
}

/// B28: a `tsconfig.json`, and a tsconfig it references, that are symlinks inside the tree
/// supply the same units and alias tables as regular files.
#[test]
fn internal_symlink_tsconfig_supplies_the_same_units_as_a_regular_file() {
    in_neutral_git_child(
        "internal_symlink_tsconfig_supplies_the_same_units_as_a_regular_file",
        || {
            let files = [
                (
                    "tsconfig.base.json",
                    r#"{"compilerOptions":{"paths":{"@/*":["./*"]}},"references":[{"path":"./packages/b"}]}"#,
                ),
                (
                    "packages/b/tsconfig.lib.json",
                    r#"{"compilerOptions":{"paths":{"@b/*":["./src/*"]}}}"#,
                ),
                ("src/index.ts", "export const index = 1;\n"),
                ("packages/b/src/b.ts", "export const b = 1;\n"),
            ];
            let links = [
                ("tsconfig.json", "tsconfig.base.json"),
                ("packages/b/tsconfig.json", "tsconfig.lib.json"),
            ];
            let regular = worktree(
                &configuration_repository(&files, &links, false),
                &manifest(""),
            );
            let linked = worktree(
                &configuration_repository(&files, &links, true),
                &manifest(""),
            );
            let aliases = |captured: &Captured| -> Vec<(String, Option<String>)> {
                captured
                    .built
                    .document
                    .summary
                    .unit_aliases
                    .iter()
                    .map(|row| (row.unit.clone(), row.alias_table.clone()))
                    .collect()
            };
            // Precondition: the reference and both alias tables are observable.
            assert_eq!(
                aliases(&regular),
                [
                    (".".to_owned(), Some("tsconfig.json".to_owned())),
                    (
                        "packages/b".to_owned(),
                        Some("packages/b/tsconfig.json".to_owned())
                    ),
                ]
            );
            assert_eq!(units_of(&linked), units_of(&regular));
            assert_eq!(aliases(&linked), aliases(&regular));
        },
    );
}

/// B28: a Cargo manifest that is a symlink inside the tree is copied for `cargo metadata`
/// as the bytes it points to.
#[test]
fn internal_symlink_cargo_manifest_supplies_the_same_units_as_a_regular_file() {
    in_neutral_git_child(
        "internal_symlink_cargo_manifest_supplies_the_same_units_as_a_regular_file",
        || {
            let files = [
                ("Cargo.toml", CARGO_WORKSPACE),
                ("Cargo.lock", CARGO_LOCK),
                ("crates/a/member.toml", CARGO_MEMBER),
                ("crates/a/src/lib.rs", "pub fn a() {}\n"),
            ];
            let links = [("crates/a/Cargo.toml", "member.toml")];
            let regular = worktree(
                &configuration_repository(&files, &links, false),
                &manifest(""),
            );
            let linked = try_capture(
                &configuration_repository(&files, &links, true),
                SnapshotKind::Worktree,
                None,
                &manifest(""),
                &BuildConfig::default(),
            )
            .unwrap_or_else(|error| panic!("worktree capture: {error}"))
            .unwrap_or_else(|error| panic!("inventory failed: {error}"));
            assert_eq!(
                regular.entry("crates/a/src/lib.rs").unit.as_deref(),
                Some("crates/a")
            );
            assert_eq!(units_of(&linked), units_of(&regular));
        },
    );
}

/// B28: a configuration symlink whose in-tree target the snapshot refuses is refused
/// with the snapshot's code, never parsed as the link's own bytes.
#[test]
fn internal_symlink_to_an_unread_target_is_refused_with_the_snapshot_code() {
    in_neutral_git_child(
        "internal_symlink_to_an_unread_target_is_refused_with_the_snapshot_code",
        || {
            let target = r#"{"main":"./src/index.ts","name":"a-package-over-the-limit"}"#;
            let repository = configuration_repository(
                &[
                    ("config/root-package.json", target),
                    ("src/index.ts", "export {};\n"),
                ],
                &[("package.json", "config/root-package.json")],
                true,
            );
            let limit = 40;
            assert!(target.len() > limit, "the target must exceed the limit");
            assert!(
                "config/root-package.json".len() <= limit,
                "the link itself must be readable"
            );
            let result = try_capture(
                &repository,
                SnapshotKind::Worktree,
                None,
                &manifest(&format!("snapshot:\n  max_file_bytes: {limit}\n")),
                &BuildConfig::default(),
            )
            .unwrap_or_else(|error| panic!("worktree capture: {error}"));
            match result {
                Err(InventoryError::Read { path, code, .. }) => {
                    assert_eq!((path.as_str(), code.as_str()), ("package.json", "oversize"));
                }
                Err(error) => panic!("unexpected inventory error: {error}"),
                Ok(captured) => panic!(
                    "the unread target was not refused; units {:?}",
                    captured.built.units
                ),
            }
        },
    );
}
