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
        let built = warrant_inventory::build(
            root,
            CapturedSnapshot {
                manifest: snapshot.manifest(),
                entries: snapshot.entries(),
                read: &read,
            },
            manifest,
            config,
        )
        .unwrap_or_else(|error| panic!("inventory failed: {error}"));
        Ok(Captured {
            built,
            requested: requested.into_inner(),
        })
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
        repository.commit_all("outside");

        let captured = worktree(&repository, &manifest(""));
        let outside = captured.entry("outside.ts");
        assert_eq!(outside.class, InventoryClass::Source);
        assert_eq!(outside.module, None);
        assert_eq!(outside.unit.as_deref(), Some("."));
        assert_eq!(outside.by, "implicit-root-unit");
        let summary = &captured.built.document.summary;
        assert_eq!(summary.unowned_source, ["outside.ts"]);
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
                let error = warrant_inventory::build(
                    root,
                    CapturedSnapshot {
                        manifest: snapshot.manifest(),
                        entries: snapshot.entries(),
                        read: &read,
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
