use std::{fs, path::Path, process::Command};

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
fn capabilities_names_implemented_and_stub_commands() {
    let output = warrant(&["capabilities"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(
        value["implemented"],
        serde_json::json!(["snapshot", "inventory", "schema", "capabilities"])
    );
    assert!(value["stubs"].as_array().expect("stub list").len() >= 19);
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stderr.contains(&0x1b));
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
