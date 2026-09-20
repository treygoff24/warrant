use super::*;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
use warrant_core::{manifest::SnapshotConfig, nouns::SnapshotKind};

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Snapshot Test",
            "-c",
            "user.email=snapshot@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join("file"), "committed\n").unwrap();
    git(dir.path(), &["add", "file"]);
    git(dir.path(), &["commit", "-qm", "root"]);
    dir
}

#[test]
fn object_snapshots_read_exact_blobs_without_touching_dirty_worktree() {
    let dir = repo();
    fs::write(dir.path().join("file"), "staged\n").unwrap();
    git(dir.path(), &["add", "file"]);
    let staged_tree = git(dir.path(), &["write-tree"]);
    let commit_tree = git(dir.path(), &["rev-parse", "HEAD^{tree}"]);
    fs::write(dir.path().join("file"), "dirty\n").unwrap();
    READ_ATTEMPTS.with(|count| count.set(0));
    for (kind, revision, tree, bytes) in [
        (
            SnapshotKind::Commit,
            Some("HEAD"),
            commit_tree.as_str(),
            b"committed\n".as_slice(),
        ),
        (
            SnapshotKind::Index,
            None,
            staged_tree.as_str(),
            b"staged\n".as_slice(),
        ),
        (
            SnapshotKind::Tree,
            Some(staged_tree.as_str()),
            staged_tree.as_str(),
            b"staged\n".as_slice(),
        ),
    ] {
        let (manifest, content) = capture(
            dir.path(),
            kind.clone(),
            revision,
            &SnapshotConfig::default(),
            |s| s.read("file"),
        )
        .unwrap();
        assert_eq!(manifest.kind, kind);
        assert_eq!(manifest.tree, format!("sha1:{tree}"));
        assert_eq!(
            manifest.repo,
            format!("sha1:{}", git(dir.path(), &["rev-parse", "HEAD"]))
        );
        assert_eq!(content, bytes);
        assert_eq!(manifest.excluded.ignored_files, None);
    }
    READ_ATTEMPTS.with(|count| assert_eq!(count.get(), 0));
    let (_, bytes) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| s.read("file"),
    )
    .unwrap();
    assert_eq!(bytes, b"dirty\n");
    READ_ATTEMPTS.with(|count| assert!(count.get() > 0));
}

#[test]
fn worktree_capture_preserves_real_index_and_tracked_ignored_files() {
    let dir = repo();
    fs::write(dir.path().join(".gitignore"), "file\nignored\n").unwrap();
    fs::write(dir.path().join("file"), "worktree\n").unwrap();
    fs::write(dir.path().join("new"), "new\n").unwrap();
    fs::write(dir.path().join("ignored"), "ignored\n").unwrap();
    let index = fs::read(dir.path().join(".git/index")).unwrap();
    READ_ATTEMPTS.with(|count| count.set(0));
    let (manifest, entries) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            assert_eq!(s.read("file")?, b"worktree\n");
            assert_eq!(s.read("new")?, b"new\n");
            assert_eq!(s.read("ignored").unwrap_err().document.code, "ignored");
            Ok(s.entries().to_vec())
        },
    )
    .unwrap();
    assert_eq!(fs::read(dir.path().join(".git/index")).unwrap(), index);
    READ_ATTEMPTS.with(|count| assert!(count.get() > 0));
    assert_eq!(manifest.excluded.ignored_files, Some(1));
    assert!(
        manifest
            .capture
            .manifest_digest
            .as_ref()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(entries.iter().any(|e| e.path == "ignored" && e.class == warrant_core::nouns::InventoryClass::Ignored));
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
    assert_eq!(manifest.kind, SnapshotKind::Worktree);
}

#[test]
fn repeated_mid_capture_rewrite_is_unstable_and_publishes_no_artifact() {
    let dir = repo();
    let cache = tempfile::tempdir().unwrap();
    let mut attempts = 0;
    let result = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            attempts += 1;
            fs::write(dir.path().join("file"), format!("rewrite {attempts}\n")).unwrap();
            s.read("file")
        },
    )
    .map(|(_, bytes)| fs::write(cache.path().join("artifact"), bytes).unwrap());
    let error = result.unwrap_err();
    assert_eq!(error.document.code, "snapshot-unstable");
    assert_eq!(error.exit_code(), 2);
    assert_eq!(attempts, 2);
    assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 0);
}

#[test]
fn one_rewrite_retakes_the_whole_snapshot_and_discards_old_results() {
    let dir = repo();
    let mut attempts = 0;
    let mut trees = Vec::new();
    let (manifest, bytes) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            attempts += 1;
            trees.push(s.manifest().tree.clone());
            if attempts == 1 {
                fs::write(dir.path().join("file"), "new stable bytes\n").unwrap();
            }
            s.read("file")
        },
    )
    .unwrap();
    assert_eq!(attempts, 2);
    assert_ne!(trees[0], trees[1]);
    assert_eq!(manifest.tree, trees[1]);
    assert_eq!(bytes, b"new stable bytes\n");
}

#[cfg(unix)]
#[test]
fn exclusions_preserve_object_identity_and_never_follow_external_links() {
    use std::os::unix::fs::symlink;
    let dir = repo();
    fs::write(dir.path().join("big"), "x".repeat(100)).unwrap();
    symlink("file", dir.path().join("internal")).unwrap();
    symlink("internal", dir.path().join("chain")).unwrap();
    symlink("../outside", dir.path().join("external")).unwrap();
    symlink("missing", dir.path().join("dangling")).unwrap();
    symlink("external", dir.path().join("external-chain")).unwrap();
    git(dir.path(), &["add", "-A"]);
    let root_commit = git(dir.path(), &["rev-parse", "HEAD"]);
    git(
        dir.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{root_commit},sub"),
        ],
    );
    let tree = git(dir.path(), &["write-tree"]);
    let config = SnapshotConfig { max_file_bytes: 32 };
    for kind in [SnapshotKind::Index, SnapshotKind::Worktree] {
        let (manifest, ()) = capture(dir.path(), kind, None, &config, |s| {
            assert_eq!(s.read("internal")?, b"file");
            assert_eq!(s.read("chain")?, b"internal");
            assert_eq!(s.mode("internal"), Some("120000"));
            for path in ["external", "dangling", "external-chain"] {
                assert_eq!(s.read(path).unwrap_err().document.code, "external-symlink");
                assert_eq!(
                    s.entries()
                        .iter()
                        .find(|e| e.path == path)
                        .unwrap()
                        .unread
                        .as_deref(),
                    Some("external-symlink")
                );
            }
            assert_eq!(s.read("big").unwrap_err().document.code, "oversize");
            assert_eq!(
                s.read("sub").unwrap_err().document.code,
                "submodule-not-descended"
            );
            assert_eq!(
                s.entries()
                    .iter()
                    .find(|e| e.path == "sub")
                    .unwrap()
                    .blob
                    .as_deref(),
                Some(root_commit.as_str())
            );
            Ok(())
        })
        .unwrap();
        assert_eq!(manifest.tree, format!("sha1:{tree}"));
        assert_eq!(manifest.excluded.oversize, 1);
        assert_eq!(manifest.excluded.submodules, 1);
    }
}

#[test]
fn input_errors_are_structured_exit_two_and_never_call_consumer() {
    let dir = repo();
    let outside = tempfile::tempdir().unwrap();
    for (path, kind, revision, code) in [
        (
            outside.path(),
            SnapshotKind::Commit,
            Some("HEAD"),
            "non-repository",
        ),
        (
            dir.path(),
            SnapshotKind::Tree,
            Some("0000000000000000000000000000000000000000"),
            "missing-tree",
        ),
        (
            dir.path(),
            SnapshotKind::Tree,
            Some("--help"),
            "missing-tree",
        ),
        (dir.path(), SnapshotKind::Tree, None, "missing-tree"),
    ] {
        let error = capture(
            path,
            kind,
            revision,
            &SnapshotConfig::default(),
            |_| -> Result<(), SnapshotError> { panic!("consumer ran on invalid input") },
        )
        .unwrap_err();
        assert_eq!(error.document.code, code);
        assert_eq!(error.document.schema_version, "warrant.error/1");
        assert_eq!(error.exit_code(), 2);
    }
    let blob = git(dir.path(), &["rev-parse", "HEAD:file"]);
    let input = format!("100644 {blob} 1\tfile\n100644 {blob} 2\tfile\n");
    crate::git::run(
        dir.path(),
        &["update-index", "--index-info"],
        None,
        Some(input.as_bytes()),
    )
    .unwrap();
    assert!(!git(dir.path(), &["ls-files", "--unmerged"]).is_empty());
    for kind in [SnapshotKind::Index, SnapshotKind::Worktree] {
        let error = capture(
            dir.path(),
            kind,
            None,
            &SnapshotConfig::default(),
            |_| -> Result<(), SnapshotError> { panic!("consumer ran on an unmerged index") },
        )
        .unwrap_err();
        assert_eq!(error.document.code, "unmerged-index");
        assert_eq!(error.exit_code(), 2);
    }
}

#[test]
fn case_collision_including_directory_prefixes_is_an_input_error() {
    let dir = repo();
    fs::create_dir(dir.path().join("Upper")).unwrap();
    fs::create_dir(dir.path().join("upper")).unwrap();
    fs::write(dir.path().join("Upper/a"), "a").unwrap();
    fs::write(dir.path().join("upper/b"), "b").unwrap();
    git(dir.path(), &["add", "-A"]);
    for kind in [SnapshotKind::Index, SnapshotKind::Worktree] {
        let error = capture(dir.path(), kind, None, &SnapshotConfig::default(), |_| {
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.document.code, "case-collision");
        assert!(error.document.reason.contains("Upper"));
        assert!(error.document.reason.contains("upper"));
        assert_eq!(error.exit_code(), 2);
    }
}

#[test]
fn sha256_repositories_and_multiple_roots_have_stable_identity() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "--object-format=sha256"]);
    fs::write(dir.path().join("file"), "same bytes").unwrap();
    git(dir.path(), &["add", "file"]);
    git(dir.path(), &["commit", "-qm", "first root"]);
    let first = git(dir.path(), &["rev-parse", "HEAD"]);
    git(dir.path(), &["checkout", "--orphan", "second"]);
    git(dir.path(), &["commit", "-qm", "second root"]);
    let second = git(dir.path(), &["rev-parse", "HEAD"]);
    let tree = git(dir.path(), &["write-tree"]);
    for kind in [
        SnapshotKind::Index,
        SnapshotKind::Worktree,
        SnapshotKind::Commit,
    ] {
        let (manifest, bytes) = capture(dir.path(), kind, None, &SnapshotConfig::default(), |s| {
            s.read("file")
        })
        .unwrap();
        assert_eq!(manifest.object_format, "sha256");
        assert_eq!(manifest.tree, format!("sha256:{tree}"));
        assert_eq!(
            manifest.repo,
            format!("sha256:{}", std::cmp::min(&first, &second))
        );
        assert_eq!(bytes, b"same bytes");
    }
}

#[test]
fn final_verification_catches_changes_even_when_consumer_does_not_read() {
    let dir = repo();
    let mut attempt = 0;
    let error = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| {
            attempt += 1;
            fs::write(dir.path().join("file"), format!("{attempt}")).unwrap();
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.document.code, "snapshot-unstable");
    assert_eq!(attempt, 2);
}

#[cfg(unix)]
#[test]
fn executable_binary_and_unusual_paths_preserve_bytes_and_modes() {
    use std::os::unix::fs::PermissionsExt;
    let dir = repo();
    let name = "odd\tline\nfile";
    fs::write(dir.path().join(name), b"\0\xff\x80").unwrap();
    fs::set_permissions(dir.path().join("file"), fs::Permissions::from_mode(0o755)).unwrap();
    let (manifest, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            assert_eq!(s.mode("file"), Some("100755"));
            assert_eq!(s.read(name)?, b"\0\xff\x80");
            Ok(())
        },
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
}

#[test]
fn linked_worktree_and_subdirectory_capture_use_repository_relative_paths() {
    let dir = repo();
    let other = tempfile::tempdir().unwrap();
    git(
        dir.path(),
        &[
            "worktree",
            "add",
            "--detach",
            other.path().to_str().unwrap(),
            "HEAD",
        ],
    );
    fs::create_dir(other.path().join("nested")).unwrap();
    fs::write(other.path().join("nested/new"), "new").unwrap();
    let index_path = git(
        other.path(),
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    );
    let before = fs::read(&index_path).unwrap();
    let (manifest, bytes) = capture(
        &other.path().join("nested"),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| s.read("nested/new"),
    )
    .unwrap();
    assert_eq!(bytes, b"new");
    assert_eq!(fs::read(&index_path).unwrap(), before);
    assert_eq!(
        manifest.repo,
        format!("sha1:{}", git(dir.path(), &["rev-parse", "HEAD"]))
    );
    let (_, committed) = capture(
        &other.path().join("nested"),
        SnapshotKind::Commit,
        None,
        &SnapshotConfig::default(),
        |s| s.read("file"),
    )
    .unwrap();
    assert_eq!(committed, b"committed\n");
}

#[test]
fn detached_head_without_branch_refs_still_has_repository_identity() {
    let dir = repo();
    let branch = git(dir.path(), &["symbolic-ref", "HEAD"]);
    let root = git(dir.path(), &["rev-parse", "HEAD"]);
    git(dir.path(), &["checkout", "--detach", "-q"]);
    git(dir.path(), &["update-ref", "-d", &branch]);
    let (manifest, ()) = capture(
        dir.path(),
        SnapshotKind::Commit,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(manifest.repo, format!("sha1:{root}"));
}

#[test]
fn ignore_configuration_digest_changes_even_when_tree_and_exclusions_do_not() {
    let dir = repo();
    let ignores = tempfile::NamedTempFile::new().unwrap();
    git(
        dir.path(),
        &[
            "config",
            "core.excludesFile",
            ignores.path().to_str().unwrap(),
        ],
    );
    fs::write(ignores.path(), "# first\n").unwrap();
    let (first, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    fs::write(ignores.path(), "# second\n").unwrap();
    let (second, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(first.tree, second.tree);
    assert_eq!(first.excluded, second.excluded);
    assert_ne!(
        first.capture.manifest_digest,
        second.capture.manifest_digest
    );
}

#[test]
fn shallow_history_cannot_claim_a_repository_root_identity() {
    let dir = repo();
    fs::write(dir.path().join("file"), "second").unwrap();
    git(dir.path(), &["commit", "-am", "second"]);
    let cloned = tempfile::tempdir().unwrap();
    git(
        cloned.path(),
        &[
            "clone",
            "--depth",
            "1",
            &format!("file://{}", dir.path().display()),
            ".",
        ],
    );
    let error = capture(
        cloned.path(),
        SnapshotKind::Commit,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap_err();
    assert_eq!(error.document.code, "missing-history");
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn deleted_files_and_file_to_directory_replacements_match_git_tree() {
    let dir = repo();
    fs::write(dir.path().join("removed"), "removed").unwrap();
    git(dir.path(), &["add", "removed"]);
    fs::remove_file(dir.path().join("removed")).unwrap();
    fs::remove_file(dir.path().join("file")).unwrap();
    fs::create_dir(dir.path().join("file")).unwrap();
    fs::write(dir.path().join("file/child"), "child").unwrap();
    let (manifest, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            assert_eq!(s.read("file/child")?, b"child");
            assert!(
                !s.entries()
                    .iter()
                    .any(|e| e.path == "removed" || e.path == "file")
            );
            Ok(())
        },
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
}

#[test]
fn sparse_checkout_entries_carry_index_ids() {
    let dir = repo();
    for name in ["kept", "dropped"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(dir.path().join(name).join("code.rs"), name).unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-qm", "directories"]);
    git(dir.path(), &["sparse-checkout", "init", "--cone"]);
    git(dir.path(), &["sparse-checkout", "set", "kept"]);
    assert!(!dir.path().join("dropped/code.rs").exists());
    let tree = git(dir.path(), &["write-tree"]);
    let (manifest, entries) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| Ok(s.entries().to_vec()),
    )
    .unwrap();
    assert_eq!(manifest.tree, format!("sha1:{tree}"));
    assert!(entries.iter().any(|entry| entry.path == "dropped/code.rs"));
}
