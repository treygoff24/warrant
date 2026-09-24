//! Exact Git snapshots. Consumers return data; callers publish only after capture succeeds.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use warrant_core::{
    manifest::SnapshotConfig,
    nouns::{
        Capture, ErrorDocument, InventoryClass, InventoryEntry, SnapshotExclusions, SnapshotKind,
        SnapshotManifest,
    },
};

mod git;
mod links;
pub mod native;
#[cfg(test)]
mod tests;
mod worktree;

#[cfg(test)]
thread_local! { static READ_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

/// Snapshot failures are input/evaluation failures (exit 2), never acceptance.
#[derive(Debug)]
pub struct SnapshotError {
    pub document: ErrorDocument,
}

impl SnapshotError {
    pub const fn exit_code(&self) -> i32 {
        2
    }
    fn new(code: &str, reason: impl Into<String>) -> Self {
        Self {
            document: ErrorDocument {
                schema_version: "warrant.error/1".into(),
                code: code.into(),
                reason: reason.into(),
                locations: Vec::new(),
                next_diagnostic: None,
            },
        }
    }
}
impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.document.code, self.document.reason)
    }
}
impl std::error::Error for SnapshotError {}
impl From<std::io::Error> for SnapshotError {
    fn from(error: std::io::Error) -> Self {
        Self::new("snapshot-io", error.to_string())
    }
}

/// Read-only view valid for one capture attempt. Paths and ids are from Git, not callers.
#[derive(Debug)]
pub struct Snapshot {
    repo: PathBuf,
    manifest: SnapshotManifest,
    entries: Vec<InventoryEntry>,
    modes: BTreeMap<String, String>,
    object_paths: BTreeSet<String>,
    file_mode: bool,
}
impl Snapshot {
    pub fn manifest(&self) -> &SnapshotManifest {
        &self.manifest
    }
    pub fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }
    pub fn mode(&self, path: &str) -> Option<&str> {
        self.modes.get(path).map(String::as_str)
    }

    /// Read exact bytes, refusing exclusions and detecting worktree drift.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, SnapshotError> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .ok_or_else(|| SnapshotError::new("missing-path", path))?;
        if let Some(reason) = &entry.unread {
            return Err(SnapshotError::new(reason, path));
        }
        let oid = entry
            .blob
            .as_deref()
            .ok_or_else(|| SnapshotError::new("ignored", path))?;
        if self.manifest.kind == SnapshotKind::Worktree && !self.object_paths.contains(path) {
            let (mode, bytes) = worktree::read_bytes(
                &self.repo,
                path,
                if self.file_mode {
                    None
                } else {
                    self.mode(path)
                },
            )
            .map_err(|error| {
                if error.document.code == "unsupported-path" {
                    error
                } else {
                    SnapshotError::new("snapshot-changed", path)
                }
            })?;
            let actual = worktree::hash(&self.repo, path, &mode, &bytes)?;
            if actual != oid || self.mode(path) != Some(mode.as_str()) {
                return Err(SnapshotError::new("snapshot-changed", path));
            }
        }
        // The identity names clean-filtered blob bytes, not raw worktree bytes.
        git::run(&self.repo, &["cat-file", "blob", oid], None, None)
    }
}

/// Evaluate one immutable source. The callback must not publish artifacts or retain
/// results between attempts: a changed worktree invalidates the entire attempt.
/// Only the successful result and its matching manifest may be published.
pub fn capture<T>(
    repo: &Path,
    kind: SnapshotKind,
    revision: Option<&str>,
    config: &SnapshotConfig,
    mut consume: impl FnMut(&Snapshot) -> Result<T, SnapshotError>,
) -> Result<(SnapshotManifest, T), SnapshotError> {
    for attempt in 0..2 {
        let result: Result<_, SnapshotError> = (|| {
            let snapshot = capture_once(repo, kind.clone(), revision, config)?;
            let result = consume(&snapshot)?;
            if kind == SnapshotKind::Worktree {
                for entry in &snapshot.entries {
                    if entry.blob.is_some() && entry.unread.is_none() {
                        snapshot.read(&entry.path)?;
                    }
                }
                let tree = worktree::tree(&snapshot.repo, snapshot.file_mode, config)?.id;
                if snapshot.manifest.tree != format!("{}:{tree}", snapshot.manifest.object_format) {
                    return Err(SnapshotError::new(
                        "snapshot-changed",
                        "worktree tree changed",
                    ));
                }
            }
            Ok((snapshot.manifest, result))
        })();
        match result {
            Err(error)
                if kind == SnapshotKind::Worktree && error.document.code == "snapshot-changed" =>
            {
                if attempt == 1 {
                    return Err(SnapshotError::new(
                        "snapshot-unstable",
                        error.document.reason,
                    ));
                }
            }
            result => return result,
        }
    }
    unreachable!("the second attempt returns a result")
}

fn capture_once(
    repo: &Path,
    kind: SnapshotKind,
    revision: Option<&str>,
    config: &SnapshotConfig,
) -> Result<Snapshot, SnapshotError> {
    let format = git::text(repo, &["rev-parse", "--show-object-format"], None).map_err(|_| {
        SnapshotError::new(
            "non-repository",
            "a Git repository with history is required",
        )
    })?;
    if git::text(repo, &["rev-parse", "--is-shallow-repository"], None)? == "true" {
        return Err(SnapshotError::new(
            "missing-history",
            "full history is required to identify the repository root",
        ));
    }
    let mut commit = None;
    let (tree, capture_kind) = match kind {
        SnapshotKind::Commit => {
            let revision = revision.unwrap_or("HEAD");
            let resolved = git::text(
                repo,
                &[
                    "rev-parse",
                    "--verify",
                    "--end-of-options",
                    &format!("{revision}^{{commit}}"),
                ],
                None,
            )
            .map_err(|_| SnapshotError::new("missing-commit", revision))?;
            commit = Some(resolved.clone());
            (
                git::text(
                    repo,
                    &[
                        "rev-parse",
                        "--verify",
                        "--end-of-options",
                        &format!("{resolved}^{{tree}}"),
                    ],
                    None,
                )?,
                "commit",
            )
        }
        SnapshotKind::Index => {
            git::check_index(repo)?;
            match native::capture(repo, SnapshotKind::Index)? {
                Some(tree) => (tree, "native-index"),
                None => (worktree::index_tree(repo)?, "index"),
            }
        }
        SnapshotKind::Tree => {
            let oid = revision
                .ok_or_else(|| SnapshotError::new("missing-tree", "a tree id is required"))?;
            if !oid.bytes().all(|b| b.is_ascii_hexdigit())
                || oid.len() != if format == "sha256" { 64 } else { 40 }
                || !matches!(
                    git::text(repo, &["cat-file", "-t", oid], None).as_deref(),
                    Ok("tree")
                )
            {
                return Err(SnapshotError::new("missing-tree", oid));
            }
            (oid.to_ascii_lowercase(), "supplied-tree")
        }
        SnapshotKind::Worktree => return worktree::capture(repo, config),
    };
    let subject = commit.as_deref().unwrap_or("HEAD");
    let roots = git::text(repo, &["rev-list", "--max-parents=0", subject], None)
        .map_err(|_| SnapshotError::new("missing-history", "cannot resolve repository roots"))?;
    let root = roots
        .lines()
        .min()
        .ok_or_else(|| SnapshotError::new("missing-history", "repository has no root commit"))?;
    let mut snapshot = Snapshot {
        repo: repo.to_owned(),
        entries: Vec::new(),
        modes: BTreeMap::new(),
        object_paths: BTreeSet::new(),
        file_mode: true,
        manifest: SnapshotManifest {
            schema_version: "warrant.snapshot/1".into(),
            repo: format!("{format}:{root}"),
            kind,
            tree: format!("{format}:{tree}"),
            object_format: format,
            commit,
            capture: Capture {
                kind: capture_kind.into(),
                manifest_digest: None,
            },
            excluded: SnapshotExclusions {
                ignored_files: None,
                ignored_count_reason: Some("not recoverable from tree".into()),
                ..Default::default()
            },
            taken_at: jiff::Timestamp::now().to_string(),
        },
    };
    load_entries(&mut snapshot, &tree, config)?;
    Ok(snapshot)
}

fn load_entries(
    snapshot: &mut Snapshot,
    tree: &str,
    config: &SnapshotConfig,
) -> Result<(), SnapshotError> {
    let output = git::run(
        &snapshot.repo,
        &["ls-tree", "--full-tree", "-rzl", tree],
        None,
        None,
    )?;
    let mut folded = BTreeMap::new();
    for record in output.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record = std::str::from_utf8(record)
            .map_err(|_| SnapshotError::new("unsupported-path", "non-UTF-8 path"))?;
        let (meta, path) = record
            .split_once('\t')
            .ok_or_else(|| SnapshotError::new("invalid-tree", "missing path"))?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        if fields.len() != 4 {
            return Err(SnapshotError::new("invalid-tree", "invalid entry"));
        }
        // Include directory prefixes: A/x and a/y also collide on case-insensitive filesystems.
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(previous) = folded.insert(prefix.to_lowercase(), prefix.clone())
                && previous != prefix
            {
                return Err(SnapshotError::new(
                    "case-collision",
                    format!("{previous} and {prefix}"),
                ));
            }
        }
        let mut entry = InventoryEntry {
            path: path.into(),
            blob: Some(fields[2].into()),
            class: InventoryClass::Unknown,
            language: None,
            unit: None,
            module: None,
            by: "snapshot".into(),
            reason: "captured".into(),
            entrypoints: Vec::new(),
            unread: None,
            generated_by: None,
            vendored_from: None,
        };
        if fields[0] == "160000" {
            entry.class = InventoryClass::Submodule;
            entry.unread = Some("submodule-not-descended".into());
            snapshot.manifest.excluded.submodules += 1;
        } else if fields[3]
            .parse::<u64>()
            .map_err(|_| SnapshotError::new("invalid-tree", "invalid size"))?
            > config.max_file_bytes
        {
            entry.class = InventoryClass::Unread;
            entry.unread = Some("oversize".into());
            snapshot.manifest.excluded.oversize += 1;
        }
        snapshot.modes.insert(path.into(), fields[0].into());
        snapshot.entries.push(entry);
    }
    links::classify(snapshot)
}
