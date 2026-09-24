use super::*;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
use warrant_core::{manifest::SnapshotConfig, nouns::SnapshotKind};

fn git(repo: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    neutralize_git_environment(&mut command);
    let output = command
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

pub(crate) fn neutralize_git_environment(command: &mut Command) {
    static EMPTY_CONFIG: std::sync::OnceLock<tempfile::NamedTempFile> = std::sync::OnceLock::new();
    let config = EMPTY_CONFIG.get_or_init(|| tempfile::NamedTempFile::new().unwrap());
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", config.path());
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
    fs::create_dir(dir.path().join("sub")).unwrap();
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
        assert_eq!(manifest.repo, format!("sha256:{second}"));
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
    let (unchanged, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(
        first.capture.manifest_digest,
        unchanged.capture.manifest_digest
    );
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

#[cfg(unix)]
#[test]
fn null_excludes_file_matches_absent_configuration() {
    let dir = repo();
    let manifest = || {
        capture(
            dir.path(),
            SnapshotKind::Worktree,
            None,
            &SnapshotConfig::default(),
            |_| Ok(()),
        )
        .map(|(m, ())| {
            (
                m.schema_version,
                m.repo,
                m.kind,
                m.tree,
                m.object_format,
                m.commit,
                m.capture,
                m.excluded,
            )
        })
        .map_err(|error| error.to_string())
    };
    let absent = manifest().unwrap();
    git(dir.path(), &["config", "core.excludesFile", "/dev/null"]);
    assert_eq!(
        manifest(),
        Ok(absent),
        "null excludes must match the absent manifest except taken_at"
    );
}

#[test]
fn directory_excludes_file_names_configuration_and_path() {
    let dir = repo();
    let excludes = tempfile::tempdir().unwrap();
    git(
        dir.path(),
        &[
            "config",
            "core.excludesFile",
            excludes.path().to_str().unwrap(),
        ],
    );
    let error = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(
        error.document.code == "snapshot-io"
            && error.document.reason.contains("core.excludesFile")
            && error
                .document
                .reason
                .contains(excludes.path().to_str().unwrap()),
        "unreadable excludes must name core.excludesFile and its path: {error}"
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

fn submodule_repo() -> TempDir {
    let dir = repo();
    let source = repo();
    git(
        dir.path(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            source.path().to_str().unwrap(),
            "sub",
        ],
    );
    git(dir.path(), &["commit", "-qam", "submodule"]);
    dir
}

fn assert_submodule_tree_matches_git(repo: &Path) {
    let (manifest, ()) = capture(
        repo,
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    let original = git::run(repo, &["ls-files", "--stage", "-z"], None, None).unwrap();
    git::run(repo, &["read-tree", "HEAD"], Some(&index), None).unwrap();
    git::run(repo, &["add", "-A"], Some(&index), None).unwrap();
    let expected = git::text(repo, &["write-tree"], Some(&index)).unwrap();
    assert_eq!(
        (
            manifest.tree,
            git::run(repo, &["ls-files", "--stage", "-z"], None, None).unwrap()
        ),
        (format!("sha1:{expected}"), original),
        "worktree gitlinks must match git add -A without changing the real index"
    );
}

#[test]
fn advanced_submodule_checkout_matches_git_add() {
    let dir = submodule_repo();
    let sub = dir.path().join("sub");
    fs::write(sub.join("file"), "advanced\n").unwrap();
    git(&sub, &["commit", "-qam", "advance"]);
    assert_submodule_tree_matches_git(dir.path());
}

#[test]
fn removed_submodule_checkout_matches_git_add() {
    let dir = submodule_repo();
    fs::remove_dir_all(dir.path().join("sub")).unwrap();
    assert_submodule_tree_matches_git(dir.path());
}

#[test]
fn empty_submodule_directory_matches_git_add() {
    let dir = submodule_repo();
    fs::remove_dir_all(dir.path().join("sub")).unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    assert_submodule_tree_matches_git(dir.path());
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

#[test]
fn clean_filter_files_hash_like_git_add() {
    let dir = repo();
    fs::write(dir.path().join(".gitattributes"), "* text=auto\n").unwrap();
    fs::write(dir.path().join("crlf"), b"first\r\nsecond\r\n").unwrap();
    assert_canonical_reads(dir.path(), "crlf");
}

#[cfg(unix)]
#[test]
fn clean_driver_reads_return_filtered_blob_bytes() {
    let dir = repo();
    git(
        dir.path(),
        &["config", "filter.canonical.clean", "tr a-z A-Z"],
    );
    fs::write(
        dir.path().join(".gitattributes"),
        "filtered filter=canonical\n",
    )
    .unwrap();
    fs::write(dir.path().join("filtered"), b"raw lowercase\n").unwrap();
    assert_canonical_reads(dir.path(), "filtered");
}

fn assert_canonical_reads(repo: &Path, path: &str) {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "canonical bytes"]);
    let tree = format!("sha1:{}", git(repo, &["write-tree"]));
    let blob = git::run(
        repo,
        &["cat-file", "blob", &format!("HEAD:{path}")],
        None,
        None,
    )
    .unwrap();
    let raw = fs::read(repo.join(path)).unwrap();
    let reads: Vec<_> = [
        SnapshotKind::Worktree,
        SnapshotKind::Index,
        SnapshotKind::Commit,
    ]
    .into_iter()
    .map(|kind| {
        let (manifest, bytes) = capture(repo, kind, None, &SnapshotConfig::default(), |s| {
            s.read(path)
        })
        .unwrap();
        (manifest.tree, bytes)
    })
    .collect();
    assert_eq!(
        (raw != blob, reads),
        (true, vec![(tree, blob); 3]),
        "all snapshot kinds must read Git's canonical blob while the raw file differs"
    );
}

#[cfg(unix)]
#[test]
fn core_filemode_false_matches_git_modes() {
    use std::os::unix::fs::PermissionsExt;
    let dir = repo();
    git(dir.path(), &["config", "core.fileMode", "false"]);
    fs::write(dir.path().join("script"), "#!/bin/sh\n").unwrap();
    fs::set_permissions(dir.path().join("script"), fs::Permissions::from_mode(0o755)).unwrap();
    let (manifest, ()) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
    capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            assert_eq!(s.mode("script"), Some("100644"));
            assert_eq!(s.read("script")?, b"#!/bin/sh\n");
            Ok(())
        },
    )
    .unwrap();
}

mod stable_identity {
    use super::*;

    #[test]
    fn identity_is_stable_across_single_branch_clones() {
        let dir = repo();
        git(dir.path(), &["branch", "-M", "main"]);
        let first = git(dir.path(), &["rev-parse", "HEAD"]);
        git(dir.path(), &["checkout", "--orphan", "docs"]);
        git(dir.path(), &["commit", "-qm", "docs root"]);
        let second = git(dir.path(), &["rev-parse", "HEAD"]);
        assert_ne!(first, second);
        // Make the unrelated root sort first so --all deterministically fails.
        git(
            dir.path(),
            &[
                "update-ref",
                "refs/heads/main",
                std::cmp::max(&first, &second),
            ],
        );
        git(
            dir.path(),
            &[
                "update-ref",
                "refs/heads/docs",
                std::cmp::min(&first, &second),
            ],
        );
        git(dir.path(), &["checkout", "main"]);
        let cloned = tempfile::tempdir().unwrap();
        git(
            cloned.path(),
            &[
                "clone",
                "--single-branch",
                "--branch",
                "main",
                dir.path().to_str().unwrap(),
                ".",
            ],
        );
        let original = capture(
            dir.path(),
            SnapshotKind::Commit,
            Some("HEAD"),
            &SnapshotConfig::default(),
            |_| Ok(()),
        )
        .unwrap()
        .0;
        let clone = capture(
            cloned.path(),
            SnapshotKind::Commit,
            Some("HEAD"),
            &SnapshotConfig::default(),
            |_| Ok(()),
        )
        .unwrap()
        .0;
        assert_eq!(original.repo, clone.repo);
        let docs = capture(
            dir.path(),
            SnapshotKind::Commit,
            Some("docs"),
            &SnapshotConfig::default(),
            |_| Ok(()),
        )
        .unwrap()
        .0;
        assert_eq!(
            docs.repo,
            format!("sha1:{}", std::cmp::min(&first, &second))
        );
    }
}

#[test]
fn created_mid_capture_file_invalidates_the_attempt() {
    let dir = repo();
    let mut attempts = 0;
    let (manifest, entries) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            attempts += 1;
            if attempts == 1 {
                fs::write(dir.path().join("created"), "new file\n").unwrap();
            }
            Ok(s.entries().to_vec())
        },
    )
    .unwrap();
    assert_eq!(attempts, 2);
    assert!(entries.iter().any(|entry| entry.path == "created"));
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
}

#[cfg(unix)]
#[test]
fn exclusions_include_untracked_oversize_and_external_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = repo();
    fs::write(dir.path().join("big-untracked"), "x".repeat(100)).unwrap();
    symlink("../outside", dir.path().join("external-untracked")).unwrap();
    let (manifest, entries) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig { max_file_bytes: 32 },
        |s| Ok(s.entries().to_vec()),
    )
    .unwrap();
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        manifest.tree,
        format!("sha1:{}", git(dir.path(), &["write-tree"]))
    );
    assert_eq!(manifest.excluded.oversize, 1);
    for (path, reason) in [
        ("big-untracked", "oversize"),
        ("external-untracked", "external-symlink"),
    ] {
        assert_eq!(
            entries
                .iter()
                .find(|e| e.path == path)
                .unwrap()
                .unread
                .as_deref(),
            Some(reason)
        );
    }
}

#[test]
fn run_stream_stripspace_round_trips_two_mib() {
    assert_stream_round_trip(false);
}

#[cfg(unix)]
#[test]
fn run_stream_drains_output_while_writing_two_mib() {
    assert_stream_round_trip(true);
}

fn assert_stream_round_trip(streaming: bool) {
    let input = b"newline-terminated line content\n".repeat(65536);
    let expected = input.clone();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let dir = repo();
        let args: &[&str] = if streaming {
            &["-c", "alias.warrant-echo=!cat", "warrant-echo"]
        } else {
            &["stripspace"]
        };
        let result = git::run_stream(dir.path(), args, None, Some(&mut input.as_slice()));
        let _ = send.send(result.map_err(|error| error.to_string()));
    });
    let actual = receive.recv_timeout(std::time::Duration::from_secs(5));
    assert!(
        actual
            .as_ref()
            .is_ok_and(|result| result.as_ref() == Ok(&expected)),
        "Git must return all input bytes before the timeout: {:?}",
        actual.as_ref().map(|result| result.as_ref().map(Vec::len))
    );
}

#[test]
fn commit_tree_resolution_uses_the_resolved_commit_id() {
    let dir = repo();
    let id = git(dir.path(), &["rev-parse", "HEAD"]);
    git::CALLS.with_borrow_mut(|calls| *calls = Some(Vec::new()));
    let (manifest, ()) = capture(
        dir.path(),
        SnapshotKind::Commit,
        Some("HEAD"),
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .unwrap();
    let calls = git::CALLS.take().unwrap();
    let tree_call = calls
        .into_iter()
        .find(|args| args.last().is_some_and(|arg| arg.ends_with("^{tree}")))
        .unwrap();
    assert_eq!(
        (manifest.commit, tree_call),
        (
            Some(id.clone()),
            vec![
                "rev-parse".into(),
                "--verify".into(),
                "--end-of-options".into(),
                format!("{id}^{{tree}}")
            ]
        ),
        "commit tree resolution must use the already resolved commit id"
    );
}

#[cfg(unix)]
#[test]
fn group_execute_without_user_execute_matches_git_add() {
    use std::os::unix::fs::PermissionsExt;
    let dir = repo();
    git(dir.path(), &["config", "core.fileMode", "true"]);
    fs::set_permissions(dir.path().join("file"), fs::Permissions::from_mode(0o654)).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    git::run(dir.path(), &["read-tree", "HEAD"], Some(&index), None).unwrap();
    git::run(dir.path(), &["add", "-A"], Some(&index), None).unwrap();
    let expected = git::text(dir.path(), &["write-tree"], Some(&index)).unwrap();
    let (manifest, mode) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| Ok(s.mode("file").unwrap().to_owned()),
    )
    .unwrap();
    let staged = git::text(dir.path(), &["ls-files", "--stage", "file"], Some(&index)).unwrap();
    assert_eq!(
        (manifest.tree, mode.as_str()),
        (
            format!("sha1:{expected}"),
            staged.split_whitespace().next().unwrap()
        ),
        "mode 0654 must match Git's user-execute-only rule"
    );
}

#[test]
fn untracked_nested_repository_matches_git_and_has_an_exclusion_reason() {
    let dir = repo();
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    git(&nested, &["init", "-q"]);
    fs::write(nested.join("file"), "nested content\n").unwrap();
    git(&nested, &["add", "file"]);
    git(&nested, &["commit", "-qm", "nested root"]);
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    git::run(dir.path(), &["read-tree", "HEAD"], Some(&index), None).unwrap();
    git::run(dir.path(), &["add", "-A"], Some(&index), None).unwrap();
    let tree = git::text(dir.path(), &["write-tree"], Some(&index)).unwrap();
    let staged = git::text(dir.path(), &["ls-files", "--stage", "nested"], Some(&index)).unwrap();
    let fields: Vec<_> = staged.split_whitespace().collect();
    let (manifest, entry) = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            Ok(s.entries()
                .iter()
                .find(|entry| entry.path == "nested")
                .map(|entry| {
                    (
                        s.mode("nested").unwrap().to_owned(),
                        entry.blob.clone(),
                        entry.class,
                        entry.reason.clone(),
                        entry.unread.clone(),
                    )
                }))
        },
    )
    .unwrap();
    assert_eq!(
        (manifest.tree, manifest.excluded.submodules, entry),
        (
            format!("sha1:{tree}"),
            1,
            Some((
                fields[0].to_owned(),
                Some(fields[1].to_owned()),
                InventoryClass::Submodule,
                "undeclared-nested-repository".into(),
                Some("submodule-not-descended".into())
            ))
        ),
        "nested repositories must match Git's gitlink and explicitly explain their exclusion"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_ancestor_is_unsupported_without_retaking_capture() {
    assert_symlinked_ancestor(false);
}

#[cfg(unix)]
#[test]
fn read_rejects_a_new_symlinked_ancestor_without_retaking_capture() {
    assert_symlinked_ancestor(true);
}

#[cfg(unix)]
fn assert_symlinked_ancestor(during_read: bool) {
    let dir = repo();
    for name in ["tracked", "sibling"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(dir.path().join(name).join("file"), "same bytes\n").unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    let replace = || {
        fs::remove_dir_all(dir.path().join("tracked")).unwrap();
        std::os::unix::fs::symlink("sibling", dir.path().join("tracked")).unwrap();
    };
    if !during_read {
        replace();
    }
    git::CALLS.with_borrow_mut(|calls| *calls = Some(Vec::new()));
    let error = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            if during_read {
                replace();
            }
            s.read("tracked/file")
        },
    )
    .unwrap_err();
    let calls = git::CALLS.take().unwrap();
    let attempts = calls
        .iter()
        .filter(|args| args.as_slice() == ["rev-parse", "--show-toplevel"])
        .count();
    assert_eq!(
        (error.document.code.as_str(), attempts),
        ("unsupported-path", 1),
        "a symlinked ancestor must be unsupported-path on the first attempt"
    );
}

#[test]
fn index_capture_succeeds_with_a_locked_unchanged_real_index() {
    let dir = repo();
    fs::write(dir.path().join("file"), "new staged content\n").unwrap();
    git(dir.path(), &["add", "file"]);
    let real_index = dir.path().join(".git/index");
    let original = fs::read(&real_index).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let scratch = temporary.path().join("index");
    fs::copy(&real_index, &scratch).unwrap();
    let tree = git::text(dir.path(), &["write-tree"], Some(&scratch)).unwrap();
    let _lock = fs::File::create_new(dir.path().join(".git/index.lock")).unwrap();
    let result = capture(
        dir.path(),
        SnapshotKind::Index,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .map(|(manifest, ())| manifest.tree)
    .map_err(|error| error.to_string());
    assert_eq!(
        (result, fs::read(&real_index).unwrap()),
        (Ok(format!("sha1:{tree}")), original),
        "index capture must succeed under index.lock without changing real index bytes"
    );
}

#[test]
fn fixture_git_ignores_ambient_global_excludes() {
    let dir = repo();
    fs::write(dir.path().join("ambient.ts"), "export {};\n").unwrap();
    git(dir.path(), &["add", "-A"]);
    assert_eq!(
        git(dir.path(), &["ls-files", "ambient.ts"]),
        "ambient.ts",
        "fixture Git must stage TypeScript files despite ambient global excludes"
    );
}

#[test]
fn unborn_nested_repository_is_recorded_without_descending() {
    let dir = repo();
    let nested = dir.path().join("vendor/nested");
    fs::create_dir_all(&nested).unwrap();
    git(&nested, &["init", "-q"]);
    fs::write(nested.join("file"), "uncommitted nested content\n").unwrap();
    let untracked = git(
        dir.path(),
        &["status", "--porcelain", "--untracked-files=all"],
    );
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    git::run(dir.path(), &["read-tree", "HEAD"], Some(&index), None).unwrap();
    let refused = git::run(dir.path(), &["add", "-A"], Some(&index), None).unwrap_err();
    let captured = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            Ok(s.entries()
                .iter()
                .filter(|entry| entry.path.starts_with("vendor/nested"))
                .map(|entry| {
                    (
                        entry.path.clone(),
                        entry.blob.clone(),
                        entry.class,
                        entry.reason.clone(),
                        entry.unread.clone(),
                    )
                })
                .collect::<Vec<_>>())
        },
    )
    .map(|(manifest, entries)| (manifest.excluded.submodules, entries))
    .map_err(|error| error.to_string());
    assert_eq!(
        (
            untracked.as_str(),
            refused
                .document
                .reason
                .contains("does not have a commit checked out"),
            captured
        ),
        (
            "?? vendor/nested/",
            true,
            Ok((
                1,
                vec![(
                    "vendor/nested".into(),
                    None,
                    InventoryClass::Submodule,
                    "undeclared-nested-repository".into(),
                    Some("submodule-not-descended".into())
                )]
            ))
        ),
        "an unborn nested repository must be recorded without a blob or descendant entries even though Git refuses staging"
    );
}

#[test]
fn unborn_repositories_created_during_capture_force_retakes() {
    let results: Vec<_> = [false, true]
        .into_iter()
        .map(|repeat| {
            let dir = repo();
            let mut attempts = 0;
            let result = capture(
                dir.path(),
                SnapshotKind::Worktree,
                None,
                &SnapshotConfig::default(),
                |s| {
                    attempts += 1;
                    if attempts == 1 || repeat {
                        let nested = dir.path().join(format!("nested-{attempts}"));
                        fs::create_dir(&nested).unwrap();
                        git(&nested, &["init", "-q"]);
                    }
                    Ok(s.entries()
                        .iter()
                        .filter(|entry| entry.reason == "undeclared-nested-repository")
                        .count())
                },
            )
            .map(|(_, count)| count)
            .map_err(|error| error.document.code);
            (result, attempts)
        })
        .collect();
    assert_eq!(
        results,
        vec![(Ok(1), 2), (Err("snapshot-unstable".into()), 2)],
        "unborn repository changes must retake once and refuse repeated changes"
    );
}

#[test]
fn declared_unborn_submodule_matches_git_add() {
    let dir = submodule_repo();
    git(&dir.path().join("sub"), &["checkout", "--orphan", "unborn"]);
    let temporary = tempfile::tempdir().unwrap();
    let index = temporary.path().join("index");
    git::run(dir.path(), &["read-tree", "HEAD"], Some(&index), None).unwrap();
    let oracle = git::run(dir.path(), &["add", "-A"], Some(&index), None).unwrap_err();
    let actual = capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |_| Ok(()),
    )
    .map(|_| ())
    .map_err(|error| (error.document.code, error.document.reason));
    assert_eq!(
        (
            oracle
                .document
                .reason
                .contains("'sub' does not have a commit checked out"),
            actual
        ),
        (true, Err(("unborn-submodule".into(), "sub".into()))),
        "declared unborn submodules must refuse like Git and name the path"
    );
}

#[cfg(unix)]
#[test]
fn public_resolve_follows_internal_chains_and_rejects_external_links() {
    let dir = repo();
    for (name, target) in [("a", "b"), ("b", "file"), ("outside", "../outside")] {
        std::os::unix::fs::symlink(target, dir.path().join(name)).unwrap();
    }
    capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            assert_eq!(
                (
                    s.resolve("a")?,
                    s.resolve("file")?,
                    s.resolve("outside").unwrap_err().document.code
                ),
                (
                    "file".into(),
                    "file".into(),
                    s.read("outside").unwrap_err().document.code
                ),
                "resolve must follow internal chains and preserve read errors"
            );
            Ok(())
        },
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn public_blob_id_applies_clean_filters() {
    let dir = repo();
    git(
        dir.path(),
        &["config", "filter.canonical.clean", "tr a-z A-Z"],
    );
    fs::write(
        dir.path().join(".gitattributes"),
        "filtered filter=canonical\n",
    )
    .unwrap();
    fs::write(dir.path().join("filtered"), b"lowercase\n").unwrap();
    capture(
        dir.path(),
        SnapshotKind::Worktree,
        None,
        &SnapshotConfig::default(),
        |s| {
            let oid = s
                .entries()
                .iter()
                .find(|entry| entry.path == "filtered")
                .unwrap()
                .blob
                .as_ref()
                .unwrap();
            assert_eq!(
                s.blob_id("filtered", b"lowercase\n")?,
                format!("{}:{oid}", s.manifest().object_format),
                "blob_id must use Git's clean-filtered identity"
            );
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn public_untracked_is_the_capture_listing_and_empty_for_commits() {
    let dir = repo();
    fs::write(dir.path().join("new"), "untracked\n").unwrap();
    let results: Vec<_> = [SnapshotKind::Worktree, SnapshotKind::Commit]
        .into_iter()
        .map(|kind| {
            capture(dir.path(), kind, None, &SnapshotConfig::default(), |s| {
                Ok(s.untracked().clone())
            })
            .unwrap()
            .1
        })
        .collect();
    assert_eq!(
        results,
        vec![BTreeSet::from(["new".into()]), BTreeSet::new()],
        "untracked must expose the worktree capture listing and be empty for object kinds"
    );
}
