use std::{fs, path::Path, process::Command};

use warrant_core::nouns::{CommandStatus, CommandsDocument, InventoryDocument};

fn warrant(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_warrant"))
        .args(args)
        .output()
        .expect("run warrant")
}

fn warrant_in(root: &Path, cache: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_warrant"))
        .args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", cache)
        .output()
        .expect("run warrant")
}

fn repository() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temp repository");
    fs::write(directory.path().join("main.rs"), "fn main() {}\n").expect("fixture source");
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "warrant@example.invalid"][..],
        &["config", "user.name", "Warrant Test"][..],
        &["add", "main.rs"][..],
        &["commit", "-qm", "fixture"][..],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(directory.path())
                .status()
                .expect("run git")
                .success()
        );
    }
    directory
}

#[test]
fn capabilities_document_and_page_walk() {
    let output = warrant(&["capabilities"]);
    assert!(output.status.success());
    // Strict core-noun deserialization checks the emitted shape without a JSON Schema validator dependency.
    let document: CommandsDocument =
        serde_json::from_slice(&output.stdout).expect("commands document");
    assert_eq!(document.schema_version, "warrant.commands/1");
    assert_eq!(
        document.implemented,
        ["snapshot", "inventory", "schema", "capabilities"]
    );
    assert_eq!(document.commands.len() as u64, document.total);
    assert!(document.schemas.contains(&"warrant.snapshot".into()));
    assert!(document.schemas.contains(&"warrant.inventory".into()));
    assert!(document.schemas.contains(&"warrant.commands".into()));
    assert!(!document.schemas.contains(&"warrant.verdict".into()));
    let mut cursor = 0;
    let mut walked = Vec::new();
    loop {
        let output = warrant(&[
            "capabilities",
            "--limit",
            "5",
            "--cursor",
            &cursor.to_string(),
        ]);
        assert!(output.status.success());
        let page: CommandsDocument = serde_json::from_slice(&output.stdout).expect("commands page");
        assert_eq!(page.total, document.total);
        assert_eq!(page.implemented, document.implemented);
        for record in &page.commands {
            assert_eq!(
                record.status,
                if document.implemented.contains(&record.name) {
                    CommandStatus::Implemented
                } else {
                    CommandStatus::Stub
                }
            );
        }
        walked.extend(page.commands);
        if !page.truncated {
            assert_eq!(page.next_cursor, None);
            break;
        }
        cursor = page.next_cursor.expect("truncated page needs cursor");
        assert_eq!(cursor, walked.len() as u64);
    }
    assert_eq!(walked, document.commands);
    let beyond = warrant(&[
        "capabilities",
        "--cursor",
        &(document.total + 1).to_string(),
    ]);
    assert_eq!(beyond.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&beyond.stderr).expect("cursor error");
    assert_eq!(error.code, "invalid-cursor");
    let end = warrant(&["capabilities", "--cursor", &document.total.to_string()]);
    assert!(end.status.success());
    let end: CommandsDocument = serde_json::from_slice(&end.stdout).expect("empty end page");
    assert!(end.commands.is_empty());
    assert!(!end.truncated);
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stderr.contains(&0x1b));
    assert!(!beyond.stderr.contains(&0x1b));
}

#[test]
fn every_stub_returns_the_error_contract() {
    for command in [
        "model",
        "query",
        "context",
        "propose",
        "check",
        "gate",
        "explain",
        "verify",
        "policy",
        "rule",
        "attest",
        "evidence",
        "instrument",
        "census",
        "map",
        "serve",
        "hook",
        "selftest",
        "self-qualify",
        "judgment",
    ] {
        let output = warrant(&[command]);
        assert_eq!(output.status.code(), Some(2), "{command}");
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).unwrap_or_else(|_| panic!("{command}"));
        assert_eq!(error["schema_version"], "warrant.error/1", "{command}");
        assert_eq!(error["code"], "not-implemented", "{command}");
        assert!(!output.stderr.contains(&0x1b), "{command}");
    }
}

#[test]
fn schema_prints_an_implemented_document_schema() {
    let output = warrant(&["schema", "warrant.snapshot"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid schema");
    assert_eq!(value["title"], "SnapshotManifest");
}

#[test]
fn snapshot_honors_warrant_yaml_limits() {
    let repository = repository();
    let cache = tempfile::tempdir().expect("temp cache");
    fs::create_dir(repository.path().join("warrant")).expect("manifest directory");
    fs::write(
        repository.path().join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 1024\n",
    )
    .expect("snapshot limits");
    fs::write(repository.path().join("large.txt"), vec![b'x'; 4096]).expect("oversize file");
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(repository.path())
            .status()
            .expect("track fixture")
            .success()
    );

    let snapshot = warrant_in(repository.path(), cache.path(), &["snapshot", "--worktree"]);
    assert!(snapshot.status.success(), "{snapshot:?}");
    let snapshot: warrant_core::nouns::SnapshotManifest =
        serde_json::from_slice(&snapshot.stdout).expect("snapshot document");
    assert_eq!(snapshot.excluded.oversize, 1);

    let inventory = warrant_in(repository.path(), cache.path(), &["inventory"]);
    assert!(inventory.status.success(), "{inventory:?}");
    let inventory: InventoryDocument =
        serde_json::from_slice(&inventory.stdout).expect("inventory document");
    let oversize: Vec<_> = inventory
        .summary
        .unread
        .iter()
        .filter(|entry| entry.reason == "oversize")
        .collect();
    assert_eq!(oversize.len() as u64, snapshot.excluded.oversize);
    assert_eq!(oversize[0].path, "large.txt");
    assert_eq!(
        inventory.summary.ignored_files,
        snapshot.excluded.ignored_files
    );
    assert_eq!(
        inventory.summary.submodules.len() as u64,
        snapshot.excluded.submodules
    );
}

#[test]
fn snapshot_and_inventory_emit_typed_json_and_atomic_cache_files() {
    let repository = repository();
    let cache = tempfile::tempdir().expect("temp cache");

    let snapshot = warrant_in(repository.path(), cache.path(), &["snapshot", "--worktree"]);
    assert!(
        snapshot.status.success(),
        "{}",
        String::from_utf8_lossy(&snapshot.stderr)
    );
    let snapshot: warrant_core::nouns::SnapshotManifest =
        serde_json::from_slice(&snapshot.stdout).expect("snapshot document");
    assert_eq!(snapshot.schema_version, "warrant.snapshot/1");

    let inventory = warrant_in(repository.path(), cache.path(), &["inventory"]);
    assert!(
        inventory.status.success(),
        "{}",
        String::from_utf8_lossy(&inventory.stderr)
    );
    let inventory: warrant_core::nouns::InventoryDocument =
        serde_json::from_slice(&inventory.stdout).expect("inventory document");
    assert_eq!(inventory.schema_version, "warrant.inventory/1");
    assert_eq!(inventory.entries.len(), 1);

    let files = walk_files(cache.path());
    assert!(files.iter().any(|path| path.ends_with("snapshot.json")));
    assert!(files.iter().any(|path| path.ends_with("inventory.json")));
    assert!(!files.iter().any(|path| path.ends_with(".tmp")));
}

#[test]
fn inventory_pages_entries_without_changing_cached_full_document() {
    let repository = repository();
    fs::write(repository.path().join("extra.rs"), "pub fn extra() {}\n").expect("second file");
    fs::write(repository.path().join("third.rs"), "pub fn third() {}\n").expect("third file");
    let cache = tempfile::tempdir().expect("temp cache");
    let full = warrant_in(repository.path(), cache.path(), &["inventory"]);
    assert!(
        full.status.success(),
        "{}",
        String::from_utf8_lossy(&full.stderr)
    );
    let full: InventoryDocument = serde_json::from_slice(&full.stdout).expect("full inventory");
    assert!(full.entries.len() >= 3);
    assert_eq!(full.total, full.entries.len() as u64);
    assert!(!full.truncated);
    let cached_path = walk_files(cache.path())
        .into_iter()
        .find(|path| path.ends_with("inventory.json"))
        .expect("cached inventory");
    let cached_before = fs::read(&cached_path).expect("cached full document");
    let cached: InventoryDocument =
        serde_json::from_slice(&cached_before).expect("cached inventory");
    assert_eq!(cached, full);

    let mut cursor = 0;
    let mut walked = Vec::new();
    loop {
        let output = warrant_in(
            repository.path(),
            cache.path(),
            &["inventory", "--limit", "1", "--cursor", &cursor.to_string()],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let page: InventoryDocument =
            serde_json::from_slice(&output.stdout).expect("inventory page");
        assert_eq!(page.total, full.total);
        assert_eq!(page.summary, full.summary);
        assert_eq!(page.entries.len(), 1);
        walked.extend(page.entries);
        if !page.truncated {
            assert_eq!(page.next_cursor, None);
            break;
        }
        cursor = page.next_cursor.expect("truncated page needs cursor");
        assert_eq!(cursor, walked.len() as u64);
    }
    assert_eq!(walked, full.entries);
    assert_eq!(
        fs::read(&cached_path).expect("cached after pagination"),
        cached_before
    );
    let beyond = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--cursor", &(full.total + 1).to_string()],
    );
    assert_eq!(beyond.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&beyond.stderr).expect("cursor error");
    assert_eq!(error.code, "invalid-cursor");
    let end = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--cursor", &full.total.to_string()],
    );
    assert!(end.status.success());
    let end: InventoryDocument = serde_json::from_slice(&end.stdout).expect("empty end page");
    assert!(end.entries.is_empty());
    assert!(!end.truncated);
}

#[test]
fn human_format_contains_the_same_data() {
    let json = warrant(&["capabilities", "--format", "json"]);
    let human = warrant(&["capabilities", "--format", "human"]);
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).expect("machine JSON");
    let human: serde_json::Value = serde_json::from_slice(&human.stdout).expect("human JSON");
    assert_eq!(json, human);
}

#[cfg(unix)]
#[test]
fn sigterm_during_capture_exits_143_without_a_partial_artifact() {
    use std::{process::Stdio, thread, time::Duration};

    let repository = repository();
    for index in 0..3_000 {
        fs::write(
            repository.path().join(format!("file-{index:04}.txt")),
            vec![b'x'; 4_096],
        )
        .expect("large fixture file");
    }
    for args in [&["add", "."][..], &["commit", "-qm", "large fixture"][..]] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(repository.path())
                .status()
                .expect("run git")
                .success()
        );
    }
    let cache = tempfile::tempdir().expect("temp cache");
    let mut child = Command::new(env!("CARGO_BIN_EXE_warrant"))
        .args(["snapshot", "--worktree"])
        .current_dir(repository.path())
        .env("XDG_CACHE_HOME", cache.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start warrant");
    thread::sleep(Duration::from_millis(25));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(child.id()).expect("pid fits i32")),
        nix::sys::signal::Signal::SIGTERM,
    )
    .expect("send SIGTERM");
    let status = child.wait().expect("wait for warrant");
    assert_eq!(status.code(), Some(143));

    let files = walk_files(cache.path());
    assert!(!files.iter().any(|path| path.ends_with(".tmp")));
    assert!(!files.iter().any(|path| path.ends_with(".json")));
}

fn walk_files(root: &Path) -> Vec<String> {
    let mut pending = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).expect("read cache") {
            let entry = entry.expect("cache entry");
            if entry.file_type().expect("entry type").is_dir() {
                pending.push(entry.path());
            } else {
                files.push(entry.path().to_string_lossy().into_owned());
            }
        }
    }
    files
}
