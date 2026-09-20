use crate::{Snapshot, SnapshotError, git, native};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use warrant_core::{
    manifest::SnapshotConfig,
    nouns::{InventoryClass, InventoryEntry, SnapshotKind},
};

pub(crate) fn capture(repo: &Path, config: &SnapshotConfig) -> Result<Snapshot, SnapshotError> {
    git::check_index(repo)?;
    let root = PathBuf::from(git::text(repo, &["rev-parse", "--show-toplevel"], None)?);
    let ignored = paths(&git::run(
        &root,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "-z",
        ],
        None,
        None,
    )?)?;
    let (tree, carried, capture_kind) = match native::capture(&root, SnapshotKind::Worktree)? {
        Some(tree) => (tree, BTreeSet::new(), "native-worktree"),
        None => {
            let (tree, carried) = portable(&root)?;
            (tree, carried, "temporary-index")
        }
    };
    let mut snapshot = crate::capture_once(&root, SnapshotKind::Tree, Some(&tree), config)?;
    snapshot.object_paths = carried;
    snapshot.manifest.kind = SnapshotKind::Worktree;
    snapshot.manifest.capture.kind = capture_kind.into();
    snapshot.manifest.excluded.ignored_files = Some(ignored.len() as u64);
    snapshot.manifest.excluded.ignored_count_reason = None;
    snapshot.manifest.capture.manifest_digest = Some(ignore_digest(&root, &snapshot, &ignored)?);
    for path in ignored {
        snapshot.entries.push(InventoryEntry {
            path,
            blob: None,
            class: InventoryClass::Ignored,
            language: None,
            unit: None,
            module: None,
            by: "git-ignore".into(),
            reason: "ignored untracked file".into(),
            entrypoints: Vec::new(),
            unread: None,
            generated_by: None,
            vendored_from: None,
        });
    }
    snapshot.entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(snapshot)
}

fn portable(repo: &Path) -> Result<(String, BTreeSet<String>), SnapshotError> {
    let temporary = TemporaryIndex::new()?;
    let index = temporary.0.join("index");
    git::run(repo, &["read-tree", "--empty"], Some(&index), None)?;
    let tracked = git::run(repo, &["ls-files", "--stage", "-t", "-z"], None, None)?;
    let mut files = BTreeMap::new();
    let mut carried = BTreeSet::new();
    for record in tracked.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record = std::str::from_utf8(record)
            .map_err(|_| SnapshotError::new("unsupported-path", "non-UTF-8 path"))?;
        let (meta, path) = record
            .split_once('\t')
            .ok_or_else(|| SnapshotError::new("invalid-index", "missing path"))?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        if fields.len() != 4 {
            return Err(SnapshotError::new("invalid-index", "invalid entry"));
        }
        files.insert(
            path.to_owned(),
            if fields[0] == "S" || fields[1] == "160000" {
                carried.insert(path.to_owned());
                Some((fields[1].to_owned(), fields[2].to_owned()))
            } else {
                None
            },
        );
    }
    for path in paths(&git::run(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        None,
        None,
    )?)? {
        files.entry(path).or_insert(None);
    }
    let mut index_info = Vec::new();
    for (path, indexed) in files {
        let (mode, oid) = if let Some(entry) = indexed {
            entry
        } else {
            // A tracked deletion is absent, not a read failure; all other errors fail closed.
            match fs::symlink_metadata(repo.join(&path)) {
                Ok(metadata) if metadata.is_dir() => continue,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
            let (mode, bytes) = read(repo, &path)?;
            let oid = git::run(
                repo,
                &["hash-object", "-w", "--stdin", "--no-filters"],
                None,
                Some(&bytes),
            )?;
            (
                mode,
                String::from_utf8(oid)
                    .map_err(|_| SnapshotError::new("git-output", "invalid blob id"))?
                    .trim()
                    .to_owned(),
            )
        };
        index_info.extend_from_slice(format!("{mode} {oid}\t{path}\0").as_bytes());
    }
    git::run(
        repo,
        &["update-index", "-z", "--index-info"],
        Some(&index),
        Some(&index_info),
    )?;
    Ok((git::text(repo, &["write-tree"], Some(&index))?, carried))
}

/// All source-byte reads, including symlink payloads and ignore inputs, pass here.
/// Symlink payloads are read without following them; non-directory ancestors fail closed.
pub(crate) fn read(repo: &Path, path: &str) -> Result<(String, Vec<u8>), SnapshotError> {
    #[cfg(test)]
    crate::READ_ATTEMPTS.with(|count| count.set(count.get() + 1));
    let relative = Path::new(path);
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(SnapshotError::new("unsupported-path", path));
    }
    let mut parent = repo.to_path_buf();
    if let Some(ancestors) = relative.parent() {
        for part in ancestors.components() {
            parent.push(part);
            if !fs::symlink_metadata(&parent)?.is_dir() {
                return Err(SnapshotError::new("snapshot-changed", path));
            }
        }
    }
    let full = repo.join(path);
    let metadata = fs::symlink_metadata(&full)?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(full)?;
        let bytes = target.as_os_str().as_encoded_bytes().to_vec();
        return Ok(("120000".into(), bytes));
    }
    if !metadata.is_file() {
        return Err(SnapshotError::new("unsupported-file", path));
    }
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    };
    #[cfg(not(unix))]
    let executable = false;
    Ok((
        if executable { "100755" } else { "100644" }.into(),
        fs::read(full)?,
    ))
}

fn paths(output: &[u8]) -> Result<BTreeSet<String>, SnapshotError> {
    output
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec())
                .map_err(|_| SnapshotError::new("unsupported-path", "non-UTF-8 path"))
        })
        .collect()
}

fn ignore_digest(
    repo: &Path,
    snapshot: &Snapshot,
    ignored: &BTreeSet<String>,
) -> Result<String, SnapshotError> {
    let mut digest = Sha256::new();
    hash_field(&mut digest, snapshot.manifest.tree.as_bytes());
    let mut configs = BTreeSet::from([PathBuf::from(".gitignore")]);
    for path in snapshot
        .entries
        .iter()
        .map(|e| &e.path)
        .chain(ignored.iter())
    {
        for parent in Path::new(path).ancestors().skip(1) {
            configs.insert(parent.join(".gitignore"));
        }
    }
    for path in configs {
        if fs::symlink_metadata(repo.join(&path)).is_ok() {
            let name = path
                .to_str()
                .ok_or_else(|| SnapshotError::new("unsupported-path", "ignore path"))?;
            let (_, bytes) = read(repo, name)?;
            hash_field(&mut digest, name.as_bytes());
            hash_field(&mut digest, &bytes);
        }
    }
    let exclude = git::text(
        repo,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/exclude",
        ],
        None,
    )?;
    let global = git::text(
        repo,
        &["config", "--path", "--get", "core.excludesFile"],
        None,
    )
    .ok()
    .or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|config| config.join("git/ignore").to_string_lossy().into_owned())
    });
    for (label, path) in [
        ("info/exclude", Some(exclude)),
        ("core.excludesFile", global),
    ] {
        if let Some(path) = path {
            let path = PathBuf::from(path);
            let path = if path.is_absolute() {
                path
            } else {
                repo.join(path)
            };
            if path.exists() {
                // Git follows explicitly configured ignore files, unlike source symlinks.
                let path = fs::canonicalize(path)?;
                let parent = path
                    .parent()
                    .ok_or_else(|| SnapshotError::new("unsupported-path", label))?;
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| SnapshotError::new("unsupported-path", label))?;
                let (_, bytes) = read(parent, name)?;
                hash_field(&mut digest, label.as_bytes());
                hash_field(&mut digest, &bytes);
            }
        }
    }
    for path in ignored {
        hash_field(&mut digest, path.as_bytes());
    }
    let hex: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!("sha256:{hex}"))
}
fn hash_field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

struct TemporaryIndex(PathBuf);
impl TemporaryIndex {
    fn new() -> Result<Self, SnapshotError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "warrant-index-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl Drop for TemporaryIndex {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
