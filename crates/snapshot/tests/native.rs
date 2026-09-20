//! S3 measurements, independent of the feature-gated shipping decision.
#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    process::Command,
};
use tempfile::TempDir;

fn git(repo: &Path, index: Option<&Path>, args: &[&str]) -> String {
    let mut command = Command::new("git");
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
    ] {
        command.env_remove(name);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .arg("-C")
        .arg(repo);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command.args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, None, &["init", "-q", "--object-format=sha1"]);
    for (key, value) in [
        ("core.autocrlf", "false"),
        ("core.fileMode", "true"),
        ("core.excludesFile", "/dev/null"),
        ("core.attributesFile", "/dev/null"),
    ] {
        git(root, None, &["config", key, value]);
    }
    fs::create_dir(root.join("src")).unwrap();
    write(root, "src/file.txt", "baseline\n");
    write(root, ".gitattributes", "*.txt text eol=lf\n");
    git(root, None, &["add", "-A"]);
    dir
}

fn write(root: &Path, path: &str, bytes: &str) {
    let path = root.join(path);
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
}

fn fixture_files(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        if directory == root && entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            fixture_files(root, &path, files);
        } else {
            files.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }
}

fn native_tree(root: &Path, index_only: bool) -> String {
    let repo = gix::open_opts(root, gix::open::Options::isolated()).unwrap();
    let mut tree = repo
        .edit_tree(gix::ObjectId::empty_tree(repo.object_hash()))
        .unwrap();
    if index_only {
        let index = repo.open_index().unwrap();
        for entry in index.entries() {
            assert_eq!(entry.stage(), gix::index::entry::Stage::Unconflicted);
            tree.upsert(
                entry.path(&index),
                entry.mode.to_tree_entry_mode().unwrap().kind(),
                entry.id,
            )
            .unwrap();
        }
    } else {
        let (mut pipeline, index) = repo.filter_pipeline(None).unwrap();
        let worktree = repo.worktree().unwrap();
        let mut excludes = worktree.excludes(None).unwrap();
        let mut files = Vec::new();
        fixture_files(root, root, &mut files);
        files.sort();
        for path in files {
            if excludes
                .at_entry(path.as_str(), None)
                .unwrap()
                .is_excluded()
            {
                continue;
            }
            let (id, kind, _) = pipeline
                .worktree_file_to_object(path.as_str().into(), &index)
                .unwrap()
                .expect("fixture contains only regular files and symlinks");
            tree.upsert(path.as_str(), kind, id).unwrap();
        }
    }
    tree.write().unwrap().to_string()
}

fn assert_parity(root: &Path, case: &str, index_only: bool) -> String {
    let original_index = fs::read(root.join(".git/index")).unwrap();
    // Capture natively first so Git cannot supply the worktree blobs under test.
    let native = native_tree(root, index_only);
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    git(root, Some(&index), &["read-tree", "--empty"]);
    git(root, Some(&index), &["add", "-A"]);
    let reference = git(root, Some(&index), &["write-tree"]);
    println!("S3 {case}: native={native} git={reference}");
    assert_eq!(original_index, fs::read(root.join(".git/index")).unwrap());
    assert_eq!(native, reference, "{case}");
    reference
}

#[test]
fn index_baseline() {
    let dir = fixture();
    let reference = assert_parity(dir.path(), "index baseline", true);
    assert_eq!(reference, git(dir.path(), None, &["write-tree"]));
}

#[test]
fn untracked() {
    let dir = fixture();
    write(dir.path(), "src/new.txt", "new\r\n");
    assert!(!git(dir.path(), None, &["ls-files"]).contains("src/new.txt"));
    assert_parity(dir.path(), "untracked", false);
}

#[test]
fn modified_tracked() {
    let dir = fixture();
    let staged = git(dir.path(), None, &["write-tree"]);
    write(dir.path(), "src/file.txt", "modified\r\n");
    let reference = assert_parity(dir.path(), "modified tracked", false);
    assert_ne!(reference, staged);
    assert_eq!(
        git(
            dir.path(),
            None,
            &["show", &format!("{reference}:src/file.txt")]
        ),
        "modified"
    );
}

#[test]
fn ignored() {
    let dir = fixture();
    write(dir.path(), ".gitignore", "ignored.txt\nignored-dir/\n");
    write(dir.path(), "ignored.txt", "excluded\n");
    fs::create_dir(dir.path().join("ignored-dir")).unwrap();
    write(dir.path(), "ignored-dir/file.txt", "excluded directory\n");
    let reference = assert_parity(dir.path(), "ignored", false);
    assert_eq!(
        git(
            dir.path(),
            None,
            &["ls-tree", "-r", "--name-only", &reference]
        ),
        ".gitattributes\n.gitignore\nsrc/file.txt"
    );
}

#[test]
fn symlinked() {
    let dir = fixture();
    symlink("src/file.txt", dir.path().join("link")).unwrap();
    let reference = assert_parity(dir.path(), "symlink", false);
    assert!(git(dir.path(), None, &["ls-tree", &reference, "link"]).starts_with("120000 "));
    assert_eq!(
        git(dir.path(), None, &["show", &format!("{reference}:link")]),
        "src/file.txt"
    );
}

#[test]
fn executable() {
    let dir = fixture();
    write(dir.path(), "run.sh", "#!/bin/sh\nexit 0\n");
    fs::set_permissions(dir.path().join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    let reference = assert_parity(dir.path(), "executable", false);
    assert!(git(dir.path(), None, &["ls-tree", &reference, "run.sh"]).starts_with("100755 "));
}
