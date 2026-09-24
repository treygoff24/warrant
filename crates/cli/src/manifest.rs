use std::{fs, path::Path};

use warrant_core::{manifest::WarrantManifest, nouns::SnapshotKind};

use crate::{error::CommandError, repository};

const MANIFEST_PATH: &str = "warrant/warrant.yaml";
const DEFAULT_MANIFEST: &str = "schema_version: warrant.manifest/1\n";

/// The worktree manifest: the file on disk, or the default when it is absent. Error
/// documents name it relative to the repository root, never by its absolute path.
pub fn load_manifest(root: &Path) -> crate::error::Result<WarrantManifest> {
    let text = match fs::read_to_string(root.join(MANIFEST_PATH)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DEFAULT_MANIFEST.into(),
        Err(error) => {
            return Err(CommandError::evaluation(
                "manifest-io",
                format!("{MANIFEST_PATH}: {error}"),
                None,
            ));
        }
    };
    parse(&text, MANIFEST_PATH.into())
}

/// The manifest a snapshot of `kind` is governed by. Object snapshots read it from the
/// same object they capture, so worktree edits cannot change an index, commit or tree
/// snapshot whose identity does not include those bytes.
pub fn load_snapshot_manifest(
    root: &Path,
    kind: &SnapshotKind,
    revision: Option<&str>,
) -> crate::error::Result<WarrantManifest> {
    let (location, blob) = match kind {
        SnapshotKind::Worktree => return load_manifest(root),
        SnapshotKind::Index => (format!(":{MANIFEST_PATH}"), index_blob(root)?),
        SnapshotKind::Commit => {
            let revision = revision.unwrap_or("HEAD");
            (
                format!("{revision}:{MANIFEST_PATH}"),
                tree_blob(root, revision)?,
            )
        }
        SnapshotKind::Tree => match revision {
            Some(tree) => (format!("{tree}:{MANIFEST_PATH}"), tree_blob(root, tree)?),
            // Capture reports the missing tree id; no object means no manifest to read.
            None => (String::new(), None),
        },
    };
    let Some(oid) = blob else {
        return parse(DEFAULT_MANIFEST, location);
    };
    let bytes = git_ok(root, &["cat-file", "blob", &oid], &location)?;
    let text = String::from_utf8(bytes).map_err(|_| {
        CommandError::evaluation("manifest-io", format!("{location}: not UTF-8"), None)
    })?;
    parse(&text, location)
}

/// The staged manifest blob. An unmerged manifest is left for capture, which refuses
/// an unmerged index before anything is read.
fn index_blob(root: &Path) -> crate::error::Result<Option<String>> {
    let output = git_ok(
        root,
        &["ls-files", "--stage", "-z", "--", MANIFEST_PATH],
        MANIFEST_PATH,
    )?;
    let mut blob = None;
    for record in records(&output) {
        let (meta, _) = record.split_once('\t').unwrap_or((record, ""));
        let fields: Vec<_> = meta.split_whitespace().collect();
        match fields.as_slice() {
            [mode, oid, "0"] => blob = Some(regular_blob(mode, oid, MANIFEST_PATH)?),
            [_, _, _] => return Ok(None),
            _ => return Err(unexpected(MANIFEST_PATH, record)),
        }
    }
    Ok(blob)
}

/// The manifest blob inside `treeish`. An unresolvable revision is left for capture to
/// report with its own error (`missing-commit`, `missing-tree`).
fn tree_blob(root: &Path, treeish: &str) -> crate::error::Result<Option<String>> {
    let peeled = format!("{treeish}^{{tree}}");
    let resolved = repository::git_in(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &peeled,
        ],
    )?;
    if !resolved.status.success() {
        return Ok(None);
    }
    let tree = String::from_utf8_lossy(&resolved.stdout).trim().to_owned();
    let location = format!("{treeish}:{MANIFEST_PATH}");
    let output = git_ok(
        root,
        &["ls-tree", "-z", "--full-tree", &tree, "--", MANIFEST_PATH],
        &location,
    )?;
    let mut blob = None;
    for record in records(&output) {
        let (meta, _) = record.split_once('\t').unwrap_or((record, ""));
        match meta.split_whitespace().collect::<Vec<_>>().as_slice() {
            [mode, _, oid] => blob = Some(regular_blob(mode, oid, &location)?),
            _ => return Err(unexpected(&location, record)),
        }
    }
    Ok(blob)
}

fn regular_blob(mode: &str, oid: &str, location: &str) -> crate::error::Result<String> {
    if matches!(mode, "100644" | "100755") {
        Ok(oid.to_owned())
    } else {
        Err(CommandError::evaluation(
            "manifest-io",
            format!("{location}: not a regular file in the snapshot (mode {mode})"),
            None,
        ))
    }
}

fn records(output: &[u8]) -> impl Iterator<Item = &str> {
    output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| std::str::from_utf8(record).unwrap_or(""))
}

fn unexpected(location: &str, record: &str) -> Box<CommandError> {
    CommandError::evaluation(
        "manifest-io",
        format!("{location}: unexpected Git record {record:?}"),
        None,
    )
}

/// Any Git failure other than an absent path is a manifest read failure.
fn git_ok(root: &Path, args: &[&str], location: &str) -> crate::error::Result<Vec<u8>> {
    let output = repository::git_in(root, args)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(CommandError::evaluation(
            "manifest-io",
            format!(
                "{location}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            None,
        ))
    }
}

fn parse(text: &str, location: String) -> crate::error::Result<WarrantManifest> {
    WarrantManifest::parse(text).map_err(|error| {
        CommandError::evaluation("invalid-manifest", error.to_string(), Some(location))
    })
}
