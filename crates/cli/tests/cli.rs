use std::{fs, path::Path, process::Command};

use warrant_core::nouns::{CommandStatus, CommandsDocument, InventoryDocument};

const STUB_COMMANDS: &[&str] = &[
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
];

fn warrant(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_warrant"))
        .args(args)
        .output()
        .expect("run warrant")
}

fn warrant_in(root: &Path, cache: &Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
    command
        .args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", cache);
    neutralize_git_environment(&mut command);
    command.output().expect("run warrant")
}

/// Git and Warrant run without the developer's system or global Git configuration.
fn neutralize_git_environment(command: &mut Command) {
    static EMPTY_CONFIG: std::sync::OnceLock<tempfile::NamedTempFile> = std::sync::OnceLock::new();
    let empty = EMPTY_CONFIG
        .get_or_init(|| tempfile::NamedTempFile::new().expect("create empty Git config file"));
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
    ] {
        command.env_remove(name);
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    neutralize_git_environment(&mut command);
    let output = command.output().expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("UTF-8 Git output")
        .trim()
        .to_owned()
}

fn commit(root: &Path, message: &str) {
    git(
        root,
        &[
            "-c",
            "user.name=Warrant Test",
            "-c",
            "user.email=warrant@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}

fn repository() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temp repository");
    fs::write(directory.path().join("main.rs"), "fn main() {}\n").expect("fixture source");
    git(directory.path(), &["init", "-q"]);
    git(directory.path(), &["add", "main.rs"]);
    commit(directory.path(), "fixture");
    directory
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON document")
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
fn capabilities_names_implemented_and_stub_commands() {
    let output = warrant(&["capabilities"]);
    assert!(output.status.success(), "{output:?}");
    let document: CommandsDocument = serde_json::from_slice(&output.stdout).expect("commands");
    assert_eq!(
        document.implemented,
        ["snapshot", "inventory", "schema", "capabilities"]
    );
    let stubs: Vec<_> = document
        .commands
        .iter()
        .filter(|record| record.status == CommandStatus::Stub)
        .map(|record| record.name.as_str())
        .collect();
    assert_eq!(stubs, STUB_COMMANDS);
}

#[test]
fn every_stub_returns_the_error_contract() {
    for command in STUB_COMMANDS {
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
    for (name, title) in [
        ("warrant.snapshot", "SnapshotManifest"),
        ("warrant.inventory", "InventoryDocument"),
    ] {
        let output = warrant(&["schema", name]);
        assert!(output.status.success(), "{output:?}");
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("valid schema");
        assert_eq!(value["title"], title);
    }
    let output = warrant(&["schema", "warrant.policy"]);
    assert_eq!(output.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&output.stderr).expect("schema error");
    assert_eq!(error.schema_version, "warrant.error/1");
    assert_eq!(error.code, "not-implemented");
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
    git(repository.path(), &["add", "."]);

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
    let repository = repository();
    let cache = tempfile::tempdir().expect("temp cache");
    for command in [
        &["capabilities"][..],
        &["snapshot", "--worktree"],
        &["inventory"],
    ] {
        let mut documents = Vec::new();
        for format in ["json", "human"] {
            let args: Vec<_> = command
                .iter()
                .copied()
                .chain(["--format", format])
                .collect();
            let output = warrant_in(repository.path(), cache.path(), &args);
            assert!(output.status.success(), "{output:?}");
            assert!(!output.stdout.contains(&0x1b));
            let mut document: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("formatted document");
            if command[0] == "snapshot" {
                // Separate captures have different clocks; all other data must match.
                let taken_at = document
                    .as_object_mut()
                    .expect("snapshot object")
                    .remove("taken_at")
                    .expect("capture timestamp");
                taken_at
                    .as_str()
                    .expect("timestamp string")
                    .parse::<jiff::Timestamp>()
                    .expect("valid timestamp");
            }
            documents.push(document);
        }
        assert_eq!(documents[0], documents[1], "{command:?}");
    }
}

#[cfg(unix)]
#[test]
fn sigterm_during_capture_exits_143_without_a_partial_artifact() {
    signal_during_capture(&[nix::sys::signal::Signal::SIGTERM], 143);
}

#[cfg(unix)]
#[test]
fn sigint_during_capture_exits_130() {
    signal_during_capture(&[nix::sys::signal::Signal::SIGINT], 130);
}

#[cfg(unix)]
#[test]
fn first_recorded_signal_decides_exit_code() {
    use nix::sys::signal::Signal::{SIGINT, SIGTERM};
    signal_during_capture(&[SIGTERM, SIGINT], 143);
}

#[cfg(unix)]
fn signal_during_capture(signals: &[nix::sys::signal::Signal], exit: i32) {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    let mut capture = PausedCapture::new(&["snapshot", "--worktree"]);
    for signal in signals {
        assert_eq!(capture.child.try_wait().expect("child liveness"), None);
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(i32::try_from(capture.child.id()).expect("pid fits i32")),
            *signal,
        )
        .expect("signal live warrant; ESRCH means the fixture failed");
        // Keep capture held while the process handles this signal before sending the next.
        let until = Instant::now() + Duration::from_millis(50);
        while Instant::now() < until {
            assert_eq!(capture.child.try_wait().expect("child liveness"), None);
            thread::sleep(Duration::from_millis(1));
        }
    }
    capture.release();
    assert_eq!(capture.wait().code(), Some(exit));
    assert!(walk_files(capture.cache.path()).is_empty());
}

#[cfg(unix)]
#[test]
fn broken_pipe_is_not_an_internal_failure() {
    use std::io::Read;

    // Git is held before inventory can write, so the pipe is certainly closed first.
    let mut capture = PausedCapture::new(&["inventory"]);
    drop(capture.child.stdout.take().expect("piped stdout"));
    capture.release();
    let status = capture.wait();
    let mut stderr = String::new();
    capture
        .child
        .stderr
        .take()
        .expect("piped stderr")
        .read_to_string(&mut stderr)
        .expect("read stderr");
    assert_eq!(status.code(), Some(0), "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
}

#[cfg(unix)]
struct PausedCapture {
    child: std::process::Child,
    cache: tempfile::TempDir,
    control: tempfile::TempDir,
    _repository: tempfile::TempDir,
}

#[cfg(unix)]
impl PausedCapture {
    fn new(args: &[&str]) -> Self {
        use std::{
            os::unix::fs::PermissionsExt,
            process::Stdio,
            thread,
            time::{Duration, Instant},
        };

        let repository = repository();
        let cache = tempfile::tempdir().expect("temp cache");
        let control = tempfile::tempdir().expect("capture control");
        let git = Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .expect("locate real git");
        assert!(git.status.success());
        let wrapper = control.path().join("git");
        fs::write(
            &wrapper,
            r#"#!/bin/sh
: > "$WARRANT_TEST_CONTROL/ready"
while [ ! -e "$WARRANT_TEST_CONTROL/release" ]; do sleep 0.01; done
"$WARRANT_TEST_GIT" "$@"
result=$?
: > "$WARRANT_TEST_CONTROL/finished"
exit "$result"
"#,
        )
        .expect("git wrapper");
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))
            .expect("executable wrapper");
        let mut paths = vec![control.path().to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").expect("PATH"),
        ));
        let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
        neutralize_git_environment(&mut command);
        let child = command
            .args(args)
            .current_dir(repository.path())
            .env("XDG_CACHE_HOME", cache.path())
            .env("PATH", std::env::join_paths(paths).expect("fixture PATH"))
            .env("WARRANT_TEST_CONTROL", control.path())
            .env(
                "WARRANT_TEST_GIT",
                String::from_utf8(git.stdout).expect("git path").trim(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start warrant");
        let mut capture = Self {
            child,
            cache,
            control,
            _repository: repository,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert_eq!(capture.child.try_wait().expect("child liveness"), None);
            // The first Git call proves signal installation finished and capture started.
            if capture.control.path().join("ready").exists() {
                break;
            }
            assert!(Instant::now() < deadline, "warrant did not enter capture");
            thread::sleep(Duration::from_millis(1));
        }
        capture
    }

    fn release(&self) {
        fs::write(self.control.path().join("release"), b"").expect("release capture");
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        use wait_timeout::ChildExt;
        self.child
            .wait_timeout(std::time::Duration::from_secs(10))
            .expect("wait for warrant")
            .expect("warrant must exit after capture resumes")
    }
}

#[cfg(unix)]
impl Drop for PausedCapture {
    fn drop(&mut self) {
        // Also unblock Git and reap our child when an assertion fails.
        self.release();
        let _ = self.child.wait();
        // A fatal-signal mutation can orphan the Git wrapper; let it finish before
        // removing its release file and repository.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while self.control.path().join("ready").exists()
            && !self.control.path().join("finished").exists()
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
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

/// Spec 4.1: `warrant snapshot --worktree` is the default when no kind is given.
#[test]
fn bare_snapshot_defaults_to_the_worktree() {
    let repository = repository();
    let cache = tempfile::tempdir().expect("temp cache");
    // An untracked file makes the worktree tree differ from the index and HEAD trees.
    fs::write(repository.path().join("untracked.rs"), "pub fn u() {}\n").expect("untracked");
    let index_tree = git(repository.path(), &["write-tree"]);

    let mut bare = json(&warrant_in(repository.path(), cache.path(), &["snapshot"]));
    let mut explicit = json(&warrant_in(
        repository.path(),
        cache.path(),
        &["snapshot", "--worktree"],
    ));
    for document in [&mut bare, &mut explicit] {
        document
            .as_object_mut()
            .expect("snapshot object")
            .remove("taken_at")
            .expect("capture timestamp");
    }
    assert_eq!(bare["kind"], "worktree");
    assert_ne!(bare["tree"], format!("sha1:{index_tree}"));
    assert_eq!(bare, explicit);

    let both = warrant_in(
        repository.path(),
        cache.path(),
        &["snapshot", "--worktree", "--index"],
    );
    assert_eq!(both.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&both.stderr).expect("usage error document");
    assert_eq!(error.code, "invalid-invocation");
}
