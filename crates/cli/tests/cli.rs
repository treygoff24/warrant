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
    // Every run re-captures, so the cached document's capture time moves; nothing else may.
    let cached_after: serde_json::Value =
        serde_json::from_slice(&fs::read(&cached_path).expect("cached after pagination"))
            .expect("cached inventory after pagination");
    assert_eq!(
        without_timestamps(cached_after),
        without_timestamps(serde_json::from_slice(&cached_before).expect("cached inventory"))
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
            let document: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("formatted document");
            // Separate captures have different clocks; all other data must match.
            let taken_at = match command[0] {
                "snapshot" => Some(&document["taken_at"]),
                "inventory" => Some(&document["snapshot"]["taken_at"]),
                _ => None,
            };
            if let Some(taken_at) = taken_at {
                taken_at
                    .as_str()
                    .expect("capture timestamp")
                    .parse::<jiff::Timestamp>()
                    .expect("valid timestamp");
            }
            documents.push(without_timestamps(document));
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

/// A terminal Ctrl-C signals the whole foreground process group: the held Git child
/// dies first and its failure is what the command sees, and the run must still exit
/// 130 as cancelled. Holding the first call lands in repository discovery; holding the
/// second lands inside snapshot capture.
#[cfg(unix)]
#[test]
fn process_group_interrupt_during_capture_exits_130() {
    use std::io::Read;

    for (args, passed) in [
        (&["snapshot", "--worktree"][..], 0),
        (&["snapshot", "--worktree"], 1),
        (&["inventory"], 0),
        (&["inventory"], 1),
    ] {
        let mut capture = PausedCapture::after_calls(args, passed);
        let group =
            nix::unistd::Pid::from_raw(i32::try_from(capture.child.id()).expect("pid fits i32"));
        nix::sys::signal::killpg(group, nix::sys::signal::Signal::SIGINT)
            .expect("signal the warrant process group");
        let status = capture.wait();
        // The group signal also killed the held Git wrapper, which never finishes.
        fs::write(capture.control.path().join("finished"), b"").expect("mark wrapper gone");
        let mut stderr = String::new();
        capture
            .child
            .stderr
            .take()
            .expect("piped stderr")
            .read_to_string(&mut stderr)
            .expect("read stderr");
        assert_eq!(
            status.code(),
            Some(130),
            "{args:?} after {passed}: {stderr}"
        );
        let error: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("cancellation document");
        assert_eq!(error["code"], "cancelled", "{args:?} after {passed}");
        assert!(
            walk_files(capture.cache.path()).is_empty(),
            "{args:?} after {passed}"
        );
    }
}

/// B30 (spec 3.6): a signal recorded after the blocked document is on its way still
/// exits 130. The document is larger than a pipe buffer, so once the first byte has been
/// read, warrant is certainly still inside the document write when the group signal
/// arrives; a signal during capture would not bind the post-write check.
#[cfg(unix)]
#[test]
fn interrupt_during_blocked_document_write_exits_130() {
    use std::{io::Read, os::unix::process::CommandExt, process::Stdio};
    use wait_timeout::ChildExt;

    let repository = repository();
    let root = repository.path();
    for index in 0..700 {
        let path = root.join(format!("src/module_{index:04}.ts"));
        fs::create_dir_all(path.parent().expect("parent")).expect("source directory");
        fs::write(path, format!("export const value{index} = {index};\n")).expect("source");
    }
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/out.txt"), "committed\n").expect("generated output");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/out.txt\"]\n      producer: \"mkdir -p gen && echo drifted > gen/out.txt\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(root, &["add", "--all"]);
    commit(root, "drifting producer");

    let cache = tempfile::tempdir().expect("temp cache");
    let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
    neutralize_git_environment(&mut command);
    let mut child = command
        .args(["inventory", "--verify-generated"])
        .current_dir(root)
        .env("XDG_CACHE_HOME", cache.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("start warrant");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut first = [0_u8; 1];
    stdout.read_exact(&mut first).expect("document started");
    let group = nix::unistd::Pid::from_raw(i32::try_from(child.id()).expect("pid fits i32"));
    nix::sys::signal::killpg(group, nix::sys::signal::Signal::SIGINT)
        .expect("signal the warrant process group");
    let mut rest = Vec::new();
    stdout.read_to_end(&mut rest).expect("read document");
    let status = child
        .wait_timeout(std::time::Duration::from_secs(30))
        .expect("wait for warrant")
        .expect("warrant exited");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("piped stderr")
        .read_to_string(&mut stderr)
        .expect("read stderr");
    // Precondition: the document outgrew the pipe, so the write was still in progress.
    assert!(
        rest.len() > 2 * 65_536,
        "document too small: {}",
        rest.len() + 1
    );
    let mut document = first.to_vec();
    document.extend(rest);
    let document: serde_json::Value =
        serde_json::from_slice(&document).expect("inventory document");
    assert_eq!(
        document["summary"]["generated_drift"][0]["path"],
        "gen/out.txt"
    );
    assert_eq!(status.code(), Some(130), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("cancellation");
    assert_eq!(error["code"], "cancelled");
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
        Self::after_calls(args, 0)
    }

    /// Hold the first Git call after `passed` calls have run normally.
    fn after_calls(args: &[&str], passed: usize) -> Self {
        use std::{
            os::unix::{fs::PermissionsExt, process::CommandExt},
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
calls=$(ls "$WARRANT_TEST_CONTROL" | grep -c '^call\.')
: > "$WARRANT_TEST_CONTROL/call.$$"
if [ "$calls" -lt "$WARRANT_TEST_PASSED_CALLS" ]; then exec "$WARRANT_TEST_GIT" "$@"; fi
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
            .env("WARRANT_TEST_PASSED_CALLS", passed.to_string())
            .env(
                "WARRANT_TEST_GIT",
                String::from_utf8(git.stdout).expect("git path").trim(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Its own process group, so a group signal reaches warrant and its Git
            // children as a terminal Ctrl-C does, and never this test process.
            .process_group(0)
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
            // The held Git call proves signal installation finished and capture started.
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

/// A repository whose manifest changes classification, units and limits, with a
/// nested directory to run from.
fn configured_repository() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temp repository");
    let root = directory.path();
    for (path, contents) in [
        (
            "warrant/warrant.yaml",
            "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 64\nintegrations:\n  lang-ts:\n    enabled: true\ninventory:\n  classes:\n    - class: doc\n      files: [\"notes/**\"]\n",
        ),
        ("package.json", "{\"workspaces\":[\"packages/*\"]}\n"),
        (
            "packages/app/package.json",
            "{\"main\":\"./src/index.ts\"}\n",
        ),
        ("packages/app/tsconfig.json", "{\"compilerOptions\":{}}\n"),
        ("packages/app/src/index.ts", "export const app = 1;\n"),
        ("packages/app/src/deep/leaf.ts", "export const leaf = 1;\n"),
        ("notes/plan.bin", "plan\n"),
    ] {
        let target = root.join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("fixture directory");
        fs::write(target, contents).expect("fixture file");
    }
    fs::write(root.join("large.txt"), vec![b'x'; 128]).expect("oversize file");
    git(root, &["init", "-q"]);
    git(root, &["add", "."]);
    commit(root, "fixture");
    directory
}

fn without_timestamps(mut document: serde_json::Value) -> serde_json::Value {
    if let Some(object) = document.as_object_mut() {
        object.remove("taken_at");
        if let Some(snapshot) = object
            .get_mut("snapshot")
            .and_then(|value| value.as_object_mut())
        {
            snapshot.remove("taken_at");
        }
    }
    document
}

#[test]
fn documents_do_not_depend_on_the_working_directory() {
    let repository = configured_repository();
    let nested = repository.path().join("packages/app/src/deep");
    let cache = tempfile::tempdir().expect("temp cache");
    for command in [&["inventory"][..], &["snapshot", "--worktree"]] {
        let from_root =
            without_timestamps(json(&warrant_in(repository.path(), cache.path(), command)));
        let from_nested = without_timestamps(json(&warrant_in(&nested, cache.path(), command)));
        assert_eq!(from_root, from_nested, "{command:?}");
    }
    // The root document really used the manifest: limits, classes, units and entrypoints.
    let inventory: InventoryDocument =
        serde_json::from_value(json(&warrant_in(&nested, cache.path(), &["inventory"])))
            .expect("inventory document");
    let entry = |path: &str| {
        inventory
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .clone()
    };
    assert_eq!(entry("large.txt").unread.as_deref(), Some("oversize"));
    assert_eq!(
        entry("notes/plan.bin").class,
        warrant_core::nouns::InventoryClass::Doc
    );
    let leaf = entry("packages/app/src/deep/leaf.ts");
    assert_eq!(leaf.class, warrant_core::nouns::InventoryClass::Source);
    assert_eq!(leaf.unit.as_deref(), Some("packages/app"));
    assert_eq!(leaf.module, None);
    assert_eq!(entry("packages/app/src/index.ts").entrypoints.len(), 1);
}

#[test]
fn non_repository_is_an_input_error() {
    let outside = tempfile::tempdir().expect("temp directory");
    let directory = outside.path().join("plain");
    fs::create_dir(&directory).expect("plain directory");
    let cache = tempfile::tempdir().expect("temp cache");
    for command in [
        &["snapshot"][..],
        &["snapshot", "--commit", "HEAD"],
        &["inventory"],
    ] {
        let mut process = Command::new(env!("CARGO_BIN_EXE_warrant"));
        process
            .args(command)
            .current_dir(&directory)
            .env("XDG_CACHE_HOME", cache.path())
            .env("GIT_CEILING_DIRECTORIES", outside.path());
        neutralize_git_environment(&mut process);
        let output = process.output().expect("run warrant");
        assert_eq!(output.status.code(), Some(2), "{command:?}");
        let error: warrant_core::nouns::ErrorDocument =
            serde_json::from_slice(&output.stderr).expect("error document");
        assert_eq!(error.code, "non-repository", "{command:?}");
        assert!(error.reason.contains("Git repository"), "{}", error.reason);
    }
}

/// A committed manifest with a 100-byte limit and one committed 128-byte file.
fn limited_repository() -> tempfile::TempDir {
    let repository = repository();
    let root = repository.path();
    fs::create_dir(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 100\n",
    )
    .expect("manifest");
    fs::write(root.join("large.txt"), vec![b'x'; 128]).expect("large file");
    git(root, &["add", "--", "warrant/warrant.yaml", "large.txt"]);
    commit(root, "limits");
    // Git's own view of the fixture: exactly one committed blob exceeds the limit.
    assert_eq!(oversize_by_git(root, "HEAD^{tree}", 100), 1);
    repository
}

/// Count blobs in a Git tree larger than `limit`, from `git ls-tree -r -l`.
fn oversize_by_git(root: &Path, tree: &str, limit: u64) -> u64 {
    git(root, &["ls-tree", "-r", "-l", "--full-tree", tree])
        .lines()
        .filter(|line| {
            let size = line.split_whitespace().nth(3).expect("ls-tree size column");
            size.parse::<u64>().is_ok_and(|size| size > limit)
        })
        .count() as u64
}

fn snapshot_document(root: &Path, cache: &Path, args: &[&str]) -> serde_json::Value {
    without_timestamps(json(&warrant_in(root, cache, args)))
}

#[test]
fn object_snapshots_ignore_the_worktree_manifest() {
    let repository = limited_repository();
    let root = repository.path();
    let cache = tempfile::tempdir().expect("temp cache");
    let head_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
    let index_tree = git(root, &["write-tree"]);
    let commands: [&[&str]; 2] = [&["snapshot", "--index"], &["snapshot", "--commit", "HEAD"]];
    let baseline: Vec<_> = commands
        .iter()
        .map(|command| snapshot_document(root, cache.path(), command))
        .collect();
    assert_eq!(baseline[0]["tree"], format!("sha1:{index_tree}"));
    assert_eq!(baseline[1]["tree"], format!("sha1:{head_tree}"));
    assert_eq!(
        baseline[0]["excluded"]["oversize"],
        oversize_by_git(root, &index_tree, 100)
    );
    assert_eq!(
        baseline[1]["excluded"]["oversize"],
        oversize_by_git(root, &head_tree, 100)
    );

    let manifest = root.join("warrant/warrant.yaml");
    for (label, change) in [
        (
            "dirty",
            Some("schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 4096\n"),
        ),
        ("invalid", Some("schema_version: [not a manifest\n")),
        ("deleted", None),
    ] {
        match change {
            Some(text) => fs::write(&manifest, text).expect("change worktree manifest"),
            None => fs::remove_file(&manifest).expect("delete worktree manifest"),
        }
        // The worktree manifest really changed: a worktree snapshot sees it.
        let worktree = warrant_in(root, cache.path(), &["snapshot", "--worktree"]);
        if label == "invalid" {
            assert_eq!(worktree.status.code(), Some(2), "{label}");
        } else {
            assert_eq!(json(&worktree)["excluded"]["oversize"], 0, "{label}");
        }
        for (command, expected) in commands.iter().zip(&baseline) {
            assert_eq!(
                &snapshot_document(root, cache.path(), command),
                expected,
                "{label} worktree manifest changed {command:?}"
            );
        }
    }
}

#[test]
fn object_snapshots_read_the_manifest_from_their_own_object() {
    let repository = limited_repository();
    let root = repository.path();
    let cache = tempfile::tempdir().expect("temp cache");
    // The manifest exists only in the commit: removed from the index and the worktree.
    git(
        root,
        &["rm", "-q", "--cached", "--", "warrant/warrant.yaml"],
    );
    fs::remove_file(root.join("warrant/warrant.yaml")).expect("delete worktree manifest");
    assert_eq!(git(root, &["ls-files", "--", "warrant/warrant.yaml"]), "");
    let head_tree = git(root, &["rev-parse", "HEAD^{tree}"]);

    let commit = snapshot_document(root, cache.path(), &["snapshot", "--commit", "HEAD"]);
    assert_eq!(commit["tree"], format!("sha1:{head_tree}"));
    assert_eq!(
        commit["excluded"]["oversize"],
        oversize_by_git(root, &head_tree, 100)
    );
    let tree = snapshot_document(root, cache.path(), &["snapshot", "--tree", &head_tree]);
    assert_eq!(tree["excluded"]["oversize"], commit["excluded"]["oversize"]);
    // The index has no manifest, so it gets the default 8 MiB limit.
    let index_tree = git(root, &["write-tree"]);
    let index = snapshot_document(root, cache.path(), &["snapshot", "--index"]);
    assert_eq!(index["tree"], format!("sha1:{index_tree}"));
    assert_eq!(
        index["excluded"]["oversize"],
        oversize_by_git(root, &index_tree, 8 * 1024 * 1024)
    );

    // A staged manifest governs the index, not the commit or the dirty worktree file.
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 16\n",
    )
    .expect("staged manifest");
    git(root, &["add", "--", "warrant/warrant.yaml"]);
    fs::write(root.join("warrant/warrant.yaml"), "not: [valid\n").expect("dirty manifest");
    let staged_tree = git(root, &["write-tree"]);
    assert_ne!(
        oversize_by_git(root, &staged_tree, 16),
        oversize_by_git(root, &staged_tree, 100),
        "the staged limit must be distinguishable from the committed one"
    );
    let index = snapshot_document(root, cache.path(), &["snapshot", "--index"]);
    assert_eq!(
        index["excluded"]["oversize"],
        oversize_by_git(root, &staged_tree, 16)
    );
    let commit_again = snapshot_document(root, cache.path(), &["snapshot", "--commit", "HEAD"]);
    assert_eq!(commit_again, commit);

    // An invalid manifest inside the object is still an invalid manifest.
    git(root, &["add", "--", "warrant/warrant.yaml"]);
    let invalid = warrant_in(root, cache.path(), &["snapshot", "--index"]);
    assert_eq!(invalid.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&invalid.stderr).expect("error document");
    assert_eq!(error.code, "invalid-manifest");
}

#[test]
fn unreadable_object_manifest_is_a_manifest_io_error() {
    let repository = limited_repository();
    let root = repository.path();
    let cache = tempfile::tempdir().expect("temp cache");
    let blob = git(root, &["rev-parse", "HEAD:warrant/warrant.yaml"]);
    fs::remove_file(root.join(".git/objects").join(&blob[..2]).join(&blob[2..]))
        .expect("remove the loose manifest object");
    // Git itself can no longer read the committed manifest.
    let mut cat = Command::new("git");
    cat.args(["cat-file", "blob", &blob]).current_dir(root);
    neutralize_git_environment(&mut cat);
    assert!(!cat.output().expect("run git").status.success());

    let output = warrant_in(root, cache.path(), &["snapshot", "--commit", "HEAD"]);
    assert_eq!(output.status.code(), Some(2));
    let error: warrant_core::nouns::ErrorDocument =
        serde_json::from_slice(&output.stderr).expect("error document");
    assert_eq!(error.code, "manifest-io", "{}", error.reason);
}

#[cfg(unix)]
#[test]
fn inventory_does_not_read_through_an_external_symlink() {
    let outside = tempfile::tempdir().expect("outside directory");
    fs::write(
        outside.path().join("package.json"),
        r#"{"main":"./main.rs","workspaces":["*"]}"#,
    )
    .expect("external package file");
    let repository = repository();
    let root = repository.path();
    std::os::unix::fs::symlink(
        outside.path().join("package.json"),
        root.join("package.json"),
    )
    .expect("external symlink");
    let cache = tempfile::tempdir().expect("temp cache");

    let inventory: InventoryDocument =
        serde_json::from_value(json(&warrant_in(root, cache.path(), &["inventory"])))
            .expect("inventory document");
    let entry = |path: &str| {
        inventory
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .clone()
    };
    assert_eq!(
        entry("package.json").unread.as_deref(),
        Some("external-symlink")
    );
    assert!(
        inventory
            .summary
            .unread
            .iter()
            .any(|item| item.path == "package.json" && item.reason == "external-symlink")
    );
    assert!(
        entry("main.rs").entrypoints.is_empty(),
        "{:?}",
        entry("main.rs")
    );
    assert!(
        !inventory
            .summary
            .unit_aliases
            .iter()
            .any(|row| row.alias_table.as_deref() == Some("package.json")),
        "{:?}",
        inventory.summary.unit_aliases
    );
}

/// A gitignored manifest is outside the snapshot tree, so the tree cannot tell two
/// limits apart; the cache key must, or the second capture overwrites the first.
#[test]
fn one_tree_under_two_size_limits_caches_two_snapshots() {
    let repository = repository();
    let root = repository.path();
    fs::write(root.join(".gitignore"), "/warrant/\n").expect("ignore the manifest");
    fs::write(root.join("large.rs"), format!("// {}\n", "x".repeat(100))).expect("large file");
    git(root, &["add", ".gitignore", "large.rs"]);
    commit(root, "large file");
    let cache = tempfile::tempdir().expect("temp cache");
    let mut oversize = Vec::new();
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    for limit in [40, 4096] {
        fs::write(
            root.join("warrant/warrant.yaml"),
            format!("schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: {limit}\n"),
        )
        .expect("manifest");
        let snapshot = json(&warrant_in(root, cache.path(), &["snapshot", "--worktree"]));
        oversize.push(snapshot["excluded"]["oversize"].clone());
    }
    assert_eq!(oversize, [serde_json::json!(1), serde_json::json!(0)]);
    let cached: Vec<_> = walk_files(cache.path())
        .into_iter()
        .filter(|path| path.ends_with("snapshot.json"))
        .collect();
    assert_eq!(cached.len(), 2, "{cached:?}");
}

/// Runs `inventory` in a fixture repository whose manifest enables both language
/// integrations and appends `inventory`, and returns the error document.
fn inventory_error(root: &Path, inventory: &str) -> serde_json::Value {
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        format!(
            "schema_version: warrant.manifest/1\nintegrations:\n  lang-ts:\n    enabled: true\n  lang-rust:\n    enabled: true\n{inventory}"
        ),
    )
    .expect("manifest");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(root, cache.path(), &["inventory"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    serde_json::from_str(stderr.trim()).expect("error document")
}

/// Spec 12.1: each inventory error has its own code, and the reason still explains it.
fn assert_inventory_error(error: &serde_json::Value, code: &str, reason: &str) {
    assert_eq!(error["code"], code, "{error}");
    assert!(
        error["reason"].as_str().expect("reason").contains(reason),
        "{error}"
    );
}

#[test]
fn inventory_error_code_invalid_glob() {
    let repository = repository();
    let error = inventory_error(
        repository.path(),
        "inventory:\n  classes:\n    - class: doc\n      files: [\"docs/[\"]\n",
    );
    assert_inventory_error(&error, "invalid-glob", "docs/[");
}

#[test]
fn inventory_error_code_classification_conflict() {
    let repository = repository();
    let error = inventory_error(
        repository.path(),
        "inventory:\n  classes:\n    - class: doc\n      files: [\"main.rs\"]\n      replaces: source\n    - class: script\n      files: [\"*.rs\"]\n      replaces: source\n",
    );
    assert_inventory_error(&error, "classification-conflict", "main.rs");
}

#[test]
fn inventory_error_code_missing_default_replacement() {
    let repository = repository();
    let error = inventory_error(
        repository.path(),
        "inventory:\n  classes:\n    - class: doc\n      files: [\"main.rs\"]\n",
    );
    assert_inventory_error(&error, "missing-default-replacement", "main.rs");
}

#[test]
fn inventory_error_code_wrong_default_replacement() {
    let repository = repository();
    let error = inventory_error(
        repository.path(),
        "inventory:\n  classes:\n    - class: doc\n      files: [\"main.rs\"]\n      replaces: test\n",
    );
    assert_inventory_error(&error, "wrong-default-replacement", "main.rs");
}

#[test]
fn inventory_error_code_nested_repository() {
    let repository = repository();
    let nested = repository.path().join("vendor/nested");
    fs::create_dir_all(&nested).expect("nested directory");
    fs::write(nested.join("lib.rs"), "pub fn nested() {}\n").expect("nested source");
    git(&nested, &["init", "-q"]);
    let error = inventory_error(repository.path(), "");
    assert_inventory_error(&error, "nested-repository", "vendor/nested");
}

#[test]
fn inventory_error_code_invalid_declaration() {
    let repository = repository();
    fs::write(repository.path().join("tsconfig.json"), "{ not json\n").expect("tsconfig");
    git(repository.path(), &["add", "tsconfig.json"]);
    commit(repository.path(), "broken tsconfig");
    let error = inventory_error(repository.path(), "");
    assert_inventory_error(&error, "invalid-declaration", "tsconfig.json");
}

/// A reproduced output inventory cannot read is an I/O failure of the inventory, not
/// drift and not a snapshot error: the producer leaves its output unreadable.
#[cfg(unix)]
#[test]
fn inventory_error_code_inventory_io() {
    let repository = repository();
    let root = repository.path();
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/out.txt"), "out\n").expect("generated output");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/out.txt\"]\n      producer: \"mkdir -p gen && printf 'out\\\\n' > gen/out.txt && chmod 000 gen/out.txt\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(root, &["add", "gen/out.txt", "warrant/warrant.yaml"]);
    commit(root, "unreadable reproduced output");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(root, cache.path(), &["inventory", "--verify-generated"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    assert_inventory_error(&error, "inventory-io", "gen/out.txt");
}

/// With neither `XDG_CACHE_HOME` nor `HOME` usable there is no cache location; the run
/// refuses instead of writing a relative `.cache` into the directory it started in.
#[test]
fn missing_cache_location_is_an_evaluation_error() {
    for xdg in [None, Some(""), Some("relative/cache")] {
        for args in [&["snapshot"][..], &["inventory"]] {
            let repository = repository();
            let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
            neutralize_git_environment(&mut command);
            command
                .args(args)
                .current_dir(repository.path())
                .env_remove("HOME")
                .env_remove("XDG_CACHE_HOME");
            if let Some(xdg) = xdg {
                command.env("XDG_CACHE_HOME", xdg);
            }
            let output = command.output().expect("run warrant");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(2), "{args:?} {xdg:?}: {stderr}");
            let error: serde_json::Value =
                serde_json::from_str(stderr.trim()).expect("error document");
            assert_eq!(error["code"], "cache-location", "{args:?} {xdg:?}");
            let reason = error["reason"].as_str().expect("reason");
            assert!(
                reason.contains("XDG_CACHE_HOME") && reason.contains("HOME"),
                "{reason}"
            );
            for relative in [".cache", "relative"] {
                assert!(
                    !repository.path().join(relative).exists(),
                    "{args:?} {xdg:?} wrote {relative}"
                );
            }
        }
    }
}

/// A repository whose reproducible producer runs `./link.sh`, a committed symlink to an
/// executable `gen.sh` that writes whether an ignored `secret.txt` is visible.
#[cfg(unix)]
fn producer_repository(producer: &str) -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository();
    let root = repository.path();
    fs::write(root.join(".gitignore"), "secret.txt\n").expect("ignore file");
    fs::write(
        root.join("gen.sh"),
        "#!/bin/sh\nmkdir -p gen\nif [ -e secret.txt ]; then echo leaked; else echo clean; fi > gen/out.txt\n",
    )
    .expect("producer script");
    fs::set_permissions(root.join("gen.sh"), fs::Permissions::from_mode(0o755))
        .expect("executable producer");
    std::os::unix::fs::symlink("gen.sh", root.join("link.sh")).expect("producer link");
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/out.txt"), "clean\n").expect("generated output");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        format!(
            "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/out.txt\"]\n      producer: \"{producer}\"\n      reproducible: true\n"
        ),
    )
    .expect("manifest");
    git(
        root,
        &[
            "add",
            ".gitignore",
            "gen.sh",
            "link.sh",
            "gen/out.txt",
            "warrant/warrant.yaml",
        ],
    );
    commit(root, "producer");
    fs::write(root.join("secret.txt"), "ignored\n").expect("ignored file on disk");
    repository
}

/// Spec 5.3: producers re-run over the snapshot's own files, in their captured modes:
/// the ignored file on disk is not in the copy, and the committed symlink and
/// executable bit survive, so the producer runs and reproduces the committed bytes.
#[cfg(unix)]
#[test]
fn verify_generated_reruns_producers_over_the_snapshot_only() {
    let repository = producer_repository("./link.sh");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--verify-generated"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inventory document");
    assert_eq!(
        document["summary"]["generated_drift"],
        serde_json::json!([])
    );
    assert_eq!(document["summary"]["ignored_files"], 1);
}

/// B26 (spec 5.3): verification compares Git blob identities. Under a clean filter the
/// captured blob holds the filtered bytes; a producer that writes the worktree's raw bytes
/// reproduces the same blob, which is not drift.
#[cfg(unix)]
#[test]
fn verify_generated_compares_blob_identities_under_a_clean_filter() {
    let repository = repository();
    let root = repository.path();
    git(root, &["config", "filter.canonical.clean", "tr a-z A-Z"]);
    fs::write(
        root.join(".gitattributes"),
        "gen/out.txt filter=canonical\n",
    )
    .expect("attributes");
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/out.txt"), "lower\n").expect("generated output");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/out.txt\"]\n      producer: \"mkdir -p gen && printf 'lower\\\\n' > gen/out.txt\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(
        root,
        &[
            "add",
            ".gitattributes",
            "gen/out.txt",
            "warrant/warrant.yaml",
        ],
    );
    commit(root, "filtered generated output");
    assert_eq!(
        git(root, &["cat-file", "blob", "HEAD:gen/out.txt"]),
        "LOWER",
        "the captured blob must hold the clean-filtered bytes"
    );
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(root, cache.path(), &["inventory", "--verify-generated"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inventory document");
    assert_eq!(
        document["summary"]["generated_drift"],
        serde_json::json!([])
    );
}

/// B26: a symlink output is a blob of its target path, as Git stores it; a producer that
/// recreates the link reproduces that blob.
#[cfg(unix)]
#[test]
fn verify_generated_hashes_symlink_outputs_by_target_path() {
    let repository = repository();
    let root = repository.path();
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/target.txt"), "target contents\n").expect("link target");
    std::os::unix::fs::symlink("target.txt", root.join("gen/link")).expect("generated link");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/link\"]\n      producer: \"mkdir -p gen && ln -s target.txt gen/link\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(
        root,
        &["add", "gen/target.txt", "gen/link", "warrant/warrant.yaml"],
    );
    commit(root, "generated link");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(root, cache.path(), &["inventory", "--verify-generated"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inventory document");
    assert_eq!(
        document["summary"]["generated_drift"],
        serde_json::json!([])
    );
}

/// Without `--verify-generated` no producer ran, so drift is unknown (null), not none.
#[cfg(unix)]
#[test]
fn inventory_without_verification_leaves_drift_unknown() {
    let repository = producer_repository("./link.sh");
    let cache = tempfile::tempdir().expect("temp cache");
    let document = json(&warrant_in(repository.path(), cache.path(), &["inventory"]));
    assert_eq!(
        document["summary"]["generated_drift"],
        serde_json::Value::Null
    );
}

#[cfg(unix)]
#[test]
fn inventory_error_code_producer_failed() {
    let repository = producer_repository("exit 3");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--verify-generated"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    assert_inventory_error(&error, "producer-failed", "exit 3");
}

/// B27: a producer's output never reaches the CLI's streams; stdout stays exactly one
/// JSON document.
#[cfg(unix)]
#[test]
fn producer_output_stays_out_of_the_document_stream() {
    let repository = producer_repository("echo progress; echo noise >&2; ./link.sh");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--verify-generated"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stdout}{stderr}");
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>();
    assert!(
        matches!(&documents, Ok(documents) if documents.len() == 1),
        "stdout is not one JSON document: {stdout}"
    );
    assert!(!stderr.contains("noise"), "{stderr}");
}

/// B27: a failing producer's reason carries its stderr, one line, with the temporary
/// copy's path replaced by `<tmp>`.
#[cfg(unix)]
#[test]
fn producer_failure_reason_carries_stderr_without_the_temporary_path() {
    let repository =
        producer_repository("pwd >&2; echo first-marker >&2; echo last-marker >&2; exit 3");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--verify-generated"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    let reason = error["reason"].as_str().expect("reason");
    assert!(
        reason.contains("<tmp>; first-marker; last-marker"),
        "{reason}"
    );
    assert!(!reason.contains('\n'), "{reason}");
    let temporary = std::env::temp_dir();
    for form in [
        temporary.clone(),
        temporary.canonicalize().expect("temp dir"),
    ] {
        let form = form.to_string_lossy();
        assert!(!reason.contains(form.as_ref()), "{reason} names {form}");
    }
}

/// B27: only the producer's last 20 stderr lines reach the reason.
#[cfg(unix)]
#[test]
fn producer_failure_reason_keeps_the_last_twenty_stderr_lines() {
    let repository = producer_repository("seq 1 25 >&2; exit 3");
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["inventory", "--verify-generated"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    let reason = error["reason"].as_str().expect("reason");
    let last: Vec<String> = (6..=25).map(|line| line.to_string()).collect();
    assert!(
        reason.ends_with(&format!("failed with status Some(3): {}", last.join("; "))),
        "{reason}"
    );
}

/// Error documents name repository files relative to the repository root, so a document
/// is the same wherever the checkout lives and never discloses the machine's layout.
#[test]
fn manifest_errors_carry_repository_relative_paths() {
    let repository = repository();
    let root = repository.path();
    let cache = tempfile::tempdir().expect("temp cache");
    let absolute = root.to_str().expect("UTF-8 temp path");

    // A directory where the manifest file belongs cannot be read: manifest-io.
    fs::create_dir_all(root.join("warrant/warrant.yaml")).expect("manifest directory");
    let output = warrant_in(root, cache.path(), &["inventory"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    assert_eq!(error["code"], "manifest-io", "{error}");
    let reason = error["reason"].as_str().expect("reason");
    assert!(reason.starts_with("warrant/warrant.yaml: "), "{reason}");
    assert!(!reason.contains(absolute), "{reason}");

    // An unparseable manifest names the same relative location.
    fs::remove_dir(root.join("warrant/warrant.yaml")).expect("remove manifest directory");
    fs::write(root.join("warrant/warrant.yaml"), "not: [valid\n").expect("invalid manifest");
    let output = warrant_in(root, cache.path(), &["snapshot"]);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).expect("error document");
    assert_eq!(error["code"], "invalid-manifest", "{error}");
    assert_eq!(error["next_diagnostic"], "warrant/warrant.yaml", "{error}");
}

/// B31: Git's stderr enters a reason relativised and on one line. In a linked worktree
/// the object store is the main checkout's absolute `.git`, outside the root, and Git
/// names it when the manifest blob is corrupt: it is reported as `<git-dir>`.
#[cfg(unix)]
#[test]
fn object_manifest_git_errors_carry_no_absolute_paths() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository();
    let main = repository.path();
    fs::create_dir_all(main.join("warrant")).expect("manifest directory");
    fs::write(
        main.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\n",
    )
    .expect("manifest");
    git(main, &["add", "warrant/warrant.yaml"]);
    commit(main, "manifest");
    let holder = tempfile::tempdir().expect("temp directory");
    let linked = holder.path().join("linked");
    let linked_text = linked.to_str().expect("UTF-8 temp path");
    git(
        main,
        &["worktree", "add", "-q", "--detach", linked_text, "HEAD"],
    );
    let oid = git(main, &["rev-parse", "HEAD:warrant/warrant.yaml"]);
    let object = main.join(".git/objects").join(&oid[..2]).join(&oid[2..]);
    fs::set_permissions(&object, fs::Permissions::from_mode(0o644)).expect("writable object");
    fs::write(&object, b"corrupt").expect("corrupt manifest blob");

    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(&linked, cache.path(), &["snapshot", "--commit", "HEAD"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    assert_eq!(error["code"], "manifest-io", "{error}");
    let reason = error["reason"].as_str().expect("reason");
    assert!(
        reason.starts_with("HEAD:warrant/warrant.yaml: "),
        "{reason}"
    );
    assert!(reason.contains("<git-dir>/objects/"), "{reason}");
    assert!(!reason.contains('\n'), "{reason}");
    for path in [main, holder.path()] {
        for form in [path.to_path_buf(), path.canonicalize().expect("temp path")] {
            let form = form.to_string_lossy();
            assert!(!reason.contains(form.as_ref()), "{reason} names {form}");
        }
    }
}

/// B31: a separate Git directory inside the root (`git init --separate-git-dir`) makes Git
/// name its objects by absolute path; the reason names them relative to the root.
#[cfg(unix)]
#[test]
fn object_manifest_git_errors_name_an_inner_git_dir_relatively() {
    use std::os::unix::fs::PermissionsExt;

    let holder = tempfile::tempdir().expect("temp repository");
    let root = holder.path().canonicalize().expect("temp path");
    let meta = root.join("meta");
    git(
        &root,
        &[
            "init",
            "-q",
            "--separate-git-dir",
            meta.to_str().expect("UTF-8"),
            ".",
        ],
    );
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\n",
    )
    .expect("manifest");
    git(&root, &["add", "warrant/warrant.yaml"]);
    commit(&root, "manifest");
    let oid = git(&root, &["rev-parse", "HEAD:warrant/warrant.yaml"]);
    let object = meta.join("objects").join(&oid[..2]).join(&oid[2..]);
    fs::set_permissions(&object, fs::Permissions::from_mode(0o644)).expect("writable object");
    fs::write(&object, b"corrupt").expect("corrupt manifest blob");

    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(&root, cache.path(), &["snapshot", "--commit", "HEAD"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    let reason = error["reason"].as_str().expect("reason");
    assert!(reason.contains("(stored in meta/objects/"), "{reason}");
    assert!(!reason.contains(root.to_str().expect("UTF-8")), "{reason}");
}

/// Cache failures name the artifact relative to the cache root.
#[test]
fn cache_errors_carry_cache_relative_paths() {
    let repository = repository();
    let holder = tempfile::tempdir().expect("temp directory");
    // A file where the cache directory belongs: creating the artifact directory fails.
    let cache = holder.path().join("cache");
    fs::write(&cache, "not a directory\n").expect("cache file");
    let output = warrant_in(repository.path(), &cache, &["snapshot"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    let reason = error["reason"].as_str().expect("reason");
    assert!(reason.starts_with("cache warrant/"), "{reason}");
    assert!(
        !reason.contains(holder.path().to_str().expect("UTF-8 temp path")),
        "{reason}"
    );
}

/// Spec 4.5: every downstream artifact carries the snapshot manifest.
#[test]
fn inventory_carries_its_snapshot_manifest() {
    let repository = repository();
    let root = repository.path();
    let cache = tempfile::tempdir().expect("temp cache");
    let snapshot = without_timestamps(json(&warrant_in(root, cache.path(), &["snapshot"])));
    let emitted = json(&warrant_in(root, cache.path(), &["inventory"]));
    let cached_path = walk_files(cache.path())
        .into_iter()
        .find(|path| path.ends_with("inventory.json"))
        .expect("cached inventory");
    let cached: serde_json::Value =
        serde_json::from_slice(&fs::read(&cached_path).expect("read cached inventory"))
            .expect("cached inventory JSON");
    for (label, document) in [("emitted", &emitted), ("cached", &cached)] {
        let carried = without_timestamps(document["snapshot"].clone());
        assert_eq!(carried["tree"], snapshot["tree"], "{label}");
        assert_eq!(carried, snapshot, "{label}");
        document["snapshot"]["taken_at"]
            .as_str()
            .expect("carried capture timestamp")
            .parse::<jiff::Timestamp>()
            .expect("valid timestamp");
    }
    assert_eq!(emitted, cached);
    // The cache key identifies the analysis, not the capture time: a second run reuses it.
    let again = json(&warrant_in(root, cache.path(), &["inventory"]));
    assert_ne!(again["snapshot"]["taken_at"], serde_json::Value::Null);
    let cached_files: Vec<_> = walk_files(cache.path())
        .into_iter()
        .filter(|path| path.ends_with("inventory.json"))
        .collect();
    assert_eq!(cached_files.len(), 1, "{cached_files:?}");
}

/// Runs `body` in a child copy of this test binary whose Git sees no system or global
/// configuration, because in-process capture shells out to Git with this process's
/// environment.
fn in_neutral_git_child(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "WARRANT_TEST_NEUTRAL_GIT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let mut command = Command::new(std::env::current_exe().expect("test binary"));
    command
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1");
    neutralize_git_environment(&mut command);
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

/// Spec 4.3: only a worktree capture can count ignored files. Index and commit
/// inventories say the count is unknown (null) rather than claiming zero.
#[test]
fn object_snapshot_inventories_leave_ignored_files_unknown() {
    in_neutral_git_child(
        "object_snapshot_inventories_leave_ignored_files_unknown",
        || {
            let repository = repository();
            let root = repository.path();
            fs::write(root.join(".gitignore"), "ignored.txt\n").expect("write .gitignore");
            git(root, &["add", ".gitignore"]);
            commit(root, "ignore");
            fs::write(root.join("ignored.txt"), "local only\n").expect("write ignored file");
            assert_eq!(
                git(
                    root,
                    &["ls-files", "--others", "--ignored", "--exclude-standard"]
                ),
                "ignored.txt"
            );
            let manifest = warrant_core::manifest::WarrantManifest::parse(
                "schema_version: warrant.manifest/1\n",
            )
            .expect("default manifest");
            for (kind, revision, expected) in [
                (warrant_core::nouns::SnapshotKind::Worktree, None, Some(1)),
                (warrant_core::nouns::SnapshotKind::Index, None, None),
                (
                    warrant_core::nouns::SnapshotKind::Commit,
                    Some("HEAD"),
                    None,
                ),
            ] {
                let (captured, document) = warrant_snapshot::capture(
                    root,
                    kind.clone(),
                    revision,
                    &manifest.snapshot,
                    |snapshot| {
                        let read = |path: &str| {
                            snapshot
                                .read(path)
                                .map_err(|error| warrant_inventory::ReadError {
                                    code: error.document.code,
                                    reason: error.document.reason,
                                })
                        };
                        let resolve = |path: &str| {
                            snapshot
                                .resolve(path)
                                .map_err(|error| warrant_inventory::ReadError {
                                    code: error.document.code,
                                    reason: error.document.reason,
                                })
                        };
                        Ok(warrant_inventory::build(
                            root,
                            warrant_inventory::CapturedSnapshot {
                                manifest: snapshot.manifest(),
                                entries: snapshot.entries(),
                                read: &read,
                                resolve: &resolve,
                                untracked: snapshot.untracked(),
                            },
                            &manifest,
                            &warrant_inventory::BuildConfig::default(),
                        )
                        .expect("inventory")
                        .document)
                    },
                )
                .unwrap_or_else(|error| panic!("{kind:?} capture: {error:?}"));
                assert_eq!(document.summary.ignored_files, expected, "{kind:?}");
                assert_eq!(
                    document.summary.ignored_files, captured.excluded.ignored_files,
                    "{kind:?} inventory and snapshot disagree on the ignored count"
                );
                assert_eq!(document.snapshot, captured, "{kind:?}");
            }
        },
    );
}

/// B29: inventory classifies against the untracked listing the capture itself took, not
/// a second listing of the live index. Git's index briefly tracks `dist/bundle.js`
/// during the second untracked listing and never again; the capture's own listing and
/// its stability check agree the file is untracked, so it is build output.
#[cfg(unix)]
#[test]
fn untracked_classification_uses_the_capture_listing() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository();
    let root = repository.path();
    fs::create_dir_all(root.join("dist")).expect("output directory");
    fs::write(root.join("dist/bundle.js"), "built();\n").expect("untracked output");
    let control = tempfile::tempdir().expect("git control");
    let real_git = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .expect("locate real git");
    assert!(real_git.status.success());
    let wrapper = control.path().join("git");
    fs::write(
        &wrapper,
        r#"#!/bin/sh
case " $* " in
  *" --ignored "*) exec "$WARRANT_TEST_GIT" "$@" ;;
  *" ls-files --others "*) ;;
  *) exec "$WARRANT_TEST_GIT" "$@" ;;
esac
: > "$WARRANT_TEST_CONTROL/listing.$$"
count=$(ls "$WARRANT_TEST_CONTROL" | grep -c '^listing\.')
if [ "$count" -ne 2 ]; then exec "$WARRANT_TEST_GIT" "$@"; fi
env -u GIT_INDEX_FILE "$WARRANT_TEST_GIT" -C "$WARRANT_TEST_ROOT" add -- dist/bundle.js || exit 97
"$WARRANT_TEST_GIT" "$@"
result=$?
env -u GIT_INDEX_FILE "$WARRANT_TEST_GIT" -C "$WARRANT_TEST_ROOT" rm -q --cached -- dist/bundle.js || exit 98
: > "$WARRANT_TEST_CONTROL/flipped"
exit "$result"
"#,
    )
    .expect("git wrapper");
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).expect("executable wrapper");
    let mut paths = vec![control.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let cache = tempfile::tempdir().expect("temp cache");
    let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
    neutralize_git_environment(&mut command);
    let output = command
        .arg("inventory")
        .current_dir(root)
        .env("XDG_CACHE_HOME", cache.path())
        .env("PATH", std::env::join_paths(paths).expect("fixture PATH"))
        .env("WARRANT_TEST_CONTROL", control.path())
        .env("WARRANT_TEST_ROOT", root)
        .env(
            "WARRANT_TEST_GIT",
            String::from_utf8(real_git.stdout).expect("git path").trim(),
        )
        .output()
        .expect("run warrant");
    let document = json(&output);
    // Precondition: the index flip happened, and the index is restored.
    assert!(
        control.path().join("flipped").exists(),
        "the second untracked listing never ran"
    );
    assert_eq!(git(root, &["ls-files", "--", "dist"]), "");
    let entry = document["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["path"] == "dist/bundle.js")
        .expect("dist/bundle.js entry");
    assert_eq!(entry["class"], "build-output", "{entry}");
}

/// B34: `--commit` resolves the revision once. A Git wrapper moves the branch to another
/// commit when the manifest blob is read, after the manifest's revision was resolved;
/// the snapshot still names the commit resolved first, under that commit's limit.
#[cfg(unix)]
#[test]
fn commit_snapshot_resolves_the_revision_once() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository();
    let root = repository.path();
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 100\n",
    )
    .expect("small-limit manifest");
    fs::write(root.join("big.txt"), "x".repeat(200)).expect("big file");
    git(root, &["add", "warrant/warrant.yaml", "big.txt"]);
    commit(root, "small limit");
    let first = git(root, &["rev-parse", "HEAD"]);
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 4096\n",
    )
    .expect("large-limit manifest");
    git(root, &["add", "warrant/warrant.yaml"]);
    commit(root, "large limit");
    let second = git(root, &["rev-parse", "HEAD"]);
    git(root, &["update-ref", "HEAD", &first]);

    let control = tempfile::tempdir().expect("git control");
    let real_git = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .expect("locate real git");
    assert!(real_git.status.success());
    let wrapper = control.path().join("git");
    fs::write(
        &wrapper,
        r#"#!/bin/sh
case " $* " in
  *" cat-file blob "*)
    if [ ! -e "$WARRANT_TEST_CONTROL/moved" ]; then
      "$WARRANT_TEST_GIT" -C "$WARRANT_TEST_ROOT" update-ref HEAD "$WARRANT_TEST_MOVE_TO" || exit 97
      : > "$WARRANT_TEST_CONTROL/moved"
    fi ;;
esac
exec "$WARRANT_TEST_GIT" "$@"
"#,
    )
    .expect("git wrapper");
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).expect("executable wrapper");
    let mut paths = vec![control.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let cache = tempfile::tempdir().expect("temp cache");
    let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
    neutralize_git_environment(&mut command);
    let output = command
        .args(["snapshot", "--commit", "HEAD"])
        .current_dir(root)
        .env("XDG_CACHE_HOME", cache.path())
        .env("PATH", std::env::join_paths(paths).expect("fixture PATH"))
        .env("WARRANT_TEST_CONTROL", control.path())
        .env("WARRANT_TEST_ROOT", root)
        .env("WARRANT_TEST_MOVE_TO", &second)
        .env(
            "WARRANT_TEST_GIT",
            String::from_utf8(real_git.stdout).expect("git path").trim(),
        )
        .output()
        .expect("run warrant");
    let document = json(&output);
    // Precondition: the branch moved during the run.
    assert!(control.path().join("moved").exists(), "the ref never moved");
    assert_eq!(git(root, &["rev-parse", "HEAD"]), second);
    let commit = document["commit"].as_str().expect("commit id");
    assert!(
        commit.ends_with(&first),
        "snapshot names {commit}, not the first-resolved {first}"
    );
    // The first commit's 100-byte limit governs its capture: only big.txt exceeds it.
    assert_eq!(document["excluded"]["oversize"], 1, "{document}");
}

/// B34: a `--commit` revision that names no commit is still reported by capture as
/// `missing-commit`, naming the revision as given.
#[test]
fn commit_snapshot_of_an_unknown_revision_is_missing_commit() {
    let repository = repository();
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(
        repository.path(),
        cache.path(),
        &["snapshot", "--commit", "no-such-revision"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    let error: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error document");
    assert_eq!(error["code"], "missing-commit", "{error}");
    assert_eq!(error["reason"], "no-such-revision", "{error}");
}

/// Run `inventory --verify-generated` and return its exit code with the drifted paths.
fn verified_drift(root: &Path) -> (Option<i32>, Vec<String>) {
    let cache = tempfile::tempdir().expect("temp cache");
    let output = warrant_in(root, cache.path(), &["inventory", "--verify-generated"]);
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "inventory document: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let paths = document["summary"]["generated_drift"]
        .as_array()
        .expect("verified drift list")
        .iter()
        .map(|row| row["path"].as_str().expect("drift path").to_owned())
        .collect();
    (output.status.code(), paths)
}

/// B35: a captured regular file that a producer recreates as a symlink has drifted, even
/// when the regular file's clean filter would map the link's target path onto the
/// captured bytes. Git stores the link's payload unfiltered, which is a different blob.
#[cfg(unix)]
#[test]
fn verify_generated_reports_a_regular_output_reproduced_as_a_symlink() {
    let repository = repository();
    let root = repository.path();
    git(root, &["config", "filter.canonical.clean", "tr a-z A-Z"]);
    fs::write(
        root.join(".gitattributes"),
        "gen/out.txt filter=canonical\n",
    )
    .expect("attributes");
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/out.txt"), "target.txt").expect("generated output");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/out.txt\"]\n      producer: \"mkdir -p gen && ln -s target.txt gen/out.txt\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(
        root,
        &[
            "add",
            ".gitattributes",
            "gen/out.txt",
            "warrant/warrant.yaml",
        ],
    );
    commit(root, "filtered regular output");
    // Oracle: Git's own blob for the reproduced link, staged under the same filter in a
    // scratch repository, is not the captured (filtered) blob.
    let captured = git(root, &["rev-parse", "HEAD:gen/out.txt"]);
    let scratch = tempfile::tempdir().expect("scratch repository");
    git(scratch.path(), &["init", "-q"]);
    git(
        scratch.path(),
        &["config", "filter.canonical.clean", "tr a-z A-Z"],
    );
    fs::write(
        scratch.path().join(".gitattributes"),
        "gen/out.txt filter=canonical\n",
    )
    .expect("scratch attributes");
    fs::create_dir_all(scratch.path().join("gen")).expect("scratch directory");
    std::os::unix::fs::symlink("target.txt", scratch.path().join("gen/out.txt"))
        .expect("scratch link");
    git(scratch.path(), &["add", "gen/out.txt"]);
    let reproduced = git(scratch.path(), &["rev-parse", ":gen/out.txt"]);
    assert_ne!(captured, reproduced, "Git stores the link unfiltered");

    let (code, drift) = verified_drift(root);
    assert_eq!(drift, ["gen/out.txt"]);
    assert_eq!(code, Some(1));
}

/// B35: a captured symlink that a producer recreates as a regular file holding the link's
/// target path has drifted: it is a different file type in Git.
#[cfg(unix)]
#[test]
fn verify_generated_reports_a_symlink_output_reproduced_as_a_regular_file() {
    let repository = repository();
    let root = repository.path();
    fs::create_dir_all(root.join("gen")).expect("generated directory");
    fs::write(root.join("gen/target.txt"), "target contents\n").expect("link target");
    std::os::unix::fs::symlink("target.txt", root.join("gen/link")).expect("generated link");
    fs::create_dir_all(root.join("warrant")).expect("manifest directory");
    fs::write(
        root.join("warrant/warrant.yaml"),
        "schema_version: warrant.manifest/1\ninventory:\n  generated:\n    - files: [\"gen/link\"]\n      producer: \"mkdir -p gen && printf target.txt > gen/link\"\n      reproducible: true\n",
    )
    .expect("manifest");
    git(
        root,
        &["add", "gen/target.txt", "gen/link", "warrant/warrant.yaml"],
    );
    commit(root, "generated link");

    let (code, drift) = verified_drift(root);
    assert_eq!(drift, ["gen/link"]);
    assert_eq!(code, Some(1));
}
