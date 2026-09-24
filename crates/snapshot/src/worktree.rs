use crate::{Snapshot, SnapshotError, git, native};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
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
    let configured_ignores = configured_ignores(&root)?;
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
    let file_mode = git::text(&root, &["config", "--bool", "--get", "core.fileMode"], None)
        .map_or(true, |value| value != "false");
    let captured = tree(&root, file_mode, config)?;
    let mut snapshot = crate::capture_once(&root, SnapshotKind::Tree, Some(&captured.id), config)?;
    if captured.kind == "temporary-index" {
        snapshot.manifest.excluded.oversize = 0;
        for entry in &mut snapshot.entries {
            if captured.oversize.contains(&entry.path) {
                entry.class = InventoryClass::Unread;
                entry.unread = Some("oversize".into());
                snapshot.manifest.excluded.oversize += 1;
            } else if entry.unread.as_deref() == Some("oversize") {
                entry.class = InventoryClass::Unknown;
                entry.unread = None;
            }
        }
        crate::links::classify(&mut snapshot)?;
    }
    for entry in &mut snapshot.entries {
        if captured.undeclared.contains(&entry.path) {
            entry.reason = "undeclared-nested-repository".into();
        }
    }
    for path in &captured.unborn {
        snapshot.manifest.excluded.submodules += 1;
        snapshot.modes.insert(path.clone(), "160000".into());
        snapshot.entries.push(InventoryEntry {
            path: path.clone(),
            blob: None,
            class: InventoryClass::Submodule,
            language: None,
            unit: None,
            module: None,
            by: "snapshot".into(),
            reason: "undeclared-nested-repository".into(),
            entrypoints: Vec::new(),
            unread: Some("submodule-not-descended".into()),
            generated_by: None,
            vendored_from: None,
        });
    }
    snapshot.object_paths = captured.carried.clone();
    snapshot.file_mode = file_mode;
    snapshot.manifest.kind = SnapshotKind::Worktree;
    snapshot.manifest.capture.kind = captured.kind.into();
    snapshot.manifest.excluded.ignored_files = Some(ignored.len() as u64);
    snapshot.manifest.excluded.ignored_count_reason = None;
    snapshot.manifest.capture.manifest_digest = Some(ignore_digest(
        &root,
        &snapshot,
        &ignored,
        &configured_ignores,
    )?);
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
    snapshot.captured_tree = Some(captured);
    Ok(snapshot)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CapturedTree {
    pub id: String,
    carried: BTreeSet<String>,
    oversize: BTreeSet<String>,
    undeclared: BTreeSet<String>,
    unborn: BTreeSet<String>,
    pub(crate) untracked: BTreeSet<String>,
    kind: &'static str,
}

pub(crate) fn index_tree(repo: &Path) -> Result<String, SnapshotError> {
    let temporary = TemporaryIndex::new()?;
    let index = temporary.0.join("index");
    let source = git::text(
        repo,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        None,
    )?;
    match fs::copy(source, &index) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            git::run(repo, &["read-tree", "--empty"], Some(&index), None)?;
        }
        Err(error) => return Err(error.into()),
    }
    // write-tree may lock and refresh the cache-tree extension even with
    // GIT_OPTIONAL_LOCKS=0. Only the private copy may be changed.
    git::text(repo, &["write-tree"], Some(&index))
}

pub(crate) fn tree(
    repo: &Path,
    file_mode: bool,
    config: &SnapshotConfig,
) -> Result<CapturedTree, SnapshotError> {
    match native::capture(repo, SnapshotKind::Worktree)? {
        Some(id) => Ok(CapturedTree {
            id,
            carried: BTreeSet::new(),
            oversize: BTreeSet::new(),
            undeclared: BTreeSet::new(),
            unborn: BTreeSet::new(),
            untracked: BTreeSet::new(),
            kind: "native-worktree",
        }),
        None => portable(repo, file_mode, config),
    }
}

fn portable(
    repo: &Path,
    file_mode: bool,
    config: &SnapshotConfig,
) -> Result<CapturedTree, SnapshotError> {
    let temporary = TemporaryIndex::new()?;
    let index = temporary.0.join("index");
    git::run(repo, &["read-tree", "--empty"], Some(&index), None)?;
    let tracked = git::run(repo, &["ls-files", "--stage", "-t", "-z"], None, None)?;
    let mut files = BTreeMap::new();
    let mut carried = BTreeSet::new();
    let mut oversize = BTreeSet::new();
    let mut undeclared = BTreeSet::new();
    let mut unborn = BTreeSet::new();
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
        let carry = fields[0] == "S";
        if carry {
            carried.insert(path.to_owned());
        }
        files.insert(
            path.to_owned(),
            Some((fields[1].to_owned(), fields[2].to_owned(), carry)),
        );
    }
    let untracked = paths(&git::run(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        None,
        None,
    )?)?;
    for path in &untracked {
        files
            .entry(path.trim_end_matches('/').to_owned())
            .or_insert(None);
    }
    let mut index_info = Vec::new();
    for (path, indexed) in files {
        let (mode, oid) = if let Some((mode, oid, true)) = &indexed {
            if mode != "160000"
                && git::text(repo, &["cat-file", "-s", oid], None)?
                    .parse::<u64>()
                    .map_err(|_| SnapshotError::new("git-output", "invalid blob size"))?
                    > config.max_file_bytes
            {
                oversize.insert(path.clone());
            }
            (mode.clone(), oid.clone())
        } else if fs::symlink_metadata(repo.join(&path)).is_ok_and(|metadata| metadata.is_dir()) {
            let submodule = repo.join(&path);
            match &indexed {
                Some((mode, oid, _)) if mode == "160000" => {
                    // Empty directories retain the recorded gitlink. Local metadata
                    // prevents rev-parse from discovering the parent repository.
                    // Git's unborn staging varies by version; Warrant always refuses.
                    let oid = if submodule.join(".git").try_exists()? {
                        nested_head(&submodule)?
                            .ok_or_else(|| SnapshotError::new("unborn-submodule", &path))?
                    } else {
                        oid.clone()
                    };
                    (mode.clone(), oid)
                }
                None if submodule.join(".git").try_exists()? => {
                    undeclared.insert(path.clone());
                    let Some(oid) = nested_head(&submodule)? else {
                        unborn.insert(path.clone());
                        continue;
                    };
                    ("160000".into(), oid)
                }
                _ => continue,
            }
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
            let indexed_mode = if file_mode {
                None
            } else {
                Some(
                    indexed
                        .as_ref()
                        .map_or("100644", |(mode, _, _)| mode.as_str()),
                )
            };
            read(repo, &path, indexed_mode, |mode, size, source| {
                let oid = if size > config.max_file_bytes {
                    oversize.insert(path.clone());
                    hash_stream(repo, &path, mode, source)?
                } else {
                    let mut bytes = Vec::new();
                    source
                        .take(config.max_file_bytes.saturating_add(1))
                        .read_to_end(&mut bytes)?;
                    if bytes.len() as u64 > config.max_file_bytes {
                        return Err(SnapshotError::new("snapshot-changed", &path));
                    }
                    hash(repo, &path, mode, &bytes)?
                };
                Ok((mode.to_owned(), oid))
            })?
        };
        index_info.extend_from_slice(format!("{mode} {oid}\t{path}\0").as_bytes());
    }
    git::run(
        repo,
        &["update-index", "-z", "--index-info"],
        Some(&index),
        Some(&index_info),
    )?;
    Ok(CapturedTree {
        id: git::text(repo, &["write-tree"], Some(&index))?,
        carried,
        oversize,
        undeclared,
        unborn,
        untracked,
        kind: "temporary-index",
    })
}

// A symbolic HEAD without a matching ref is unborn; other failures propagate.
fn nested_head(repo: &Path) -> Result<Option<String>, SnapshotError> {
    match git::text(repo, &["rev-parse", "--verify", "HEAD"], None) {
        Ok(oid) => Ok(Some(oid)),
        Err(error) => {
            let reference = git::text(repo, &["symbolic-ref", "HEAD"], None)?;
            let refs = git::text(
                repo,
                &["for-each-ref", "--format=%(refname)", &reference],
                None,
            )?;
            if refs.lines().any(|name| name == reference) {
                Err(error)
            } else {
                Ok(None)
            }
        }
    }
}

// Git does not apply clean filters to symlink payloads.
pub(crate) fn hash(
    repo: &Path,
    path: &str,
    mode: &str,
    bytes: &[u8],
) -> Result<String, SnapshotError> {
    hash_stream(repo, path, mode, &mut &bytes[..])
}

fn hash_stream(
    repo: &Path,
    path: &str,
    mode: &str,
    source: &mut dyn Read,
) -> Result<String, SnapshotError> {
    let filter = if mode == "120000" {
        "--no-filters".into()
    } else {
        format!("--path={path}")
    };
    let oid = git::run_stream(
        repo,
        &["hash-object", "-w", "--stdin", &filter],
        None,
        Some(source),
    )?;
    Ok(String::from_utf8(oid)
        .map_err(|_| SnapshotError::new("git-output", "invalid blob id"))?
        .trim()
        .to_owned())
}

pub(crate) fn read_bytes(
    repo: &Path,
    path: &str,
    indexed_mode: Option<&str>,
) -> Result<(String, Vec<u8>), SnapshotError> {
    read(repo, path, indexed_mode, |mode, _, source| {
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        Ok((mode.to_owned(), bytes))
    })
}

/// Source-byte reads, including symlink payloads and repository ignore inputs, pass here.
/// Symlink payloads are read without following them; non-directory ancestors fail closed.
fn read<T>(
    repo: &Path,
    path: &str,
    indexed_mode: Option<&str>,
    consume: impl FnOnce(&str, u64, &mut dyn Read) -> Result<T, SnapshotError>,
) -> Result<T, SnapshotError> {
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
            let metadata = fs::symlink_metadata(&parent)?;
            if metadata.file_type().is_symlink() {
                return Err(SnapshotError::new("unsupported-path", path));
            }
            if !metadata.is_dir() {
                return Err(SnapshotError::new("snapshot-changed", path));
            }
        }
    }
    let full = repo.join(path);
    let metadata = fs::symlink_metadata(&full)?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(full)?;
        let bytes = target.as_os_str().as_encoded_bytes().to_vec();
        return consume("120000", bytes.len() as u64, &mut bytes.as_slice());
    }
    if !metadata.is_file() {
        return Err(SnapshotError::new("unsupported-file", path));
    }
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o100 != 0
    };
    #[cfg(not(unix))]
    let executable = false;
    let mode = indexed_mode.map_or(if executable { "100755" } else { "100644" }, |mode| {
        if mode == "100755" { "100755" } else { "100644" }
    });
    consume(mode, metadata.len(), &mut fs::File::open(full)?)
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
    configured_ignores: &[(&str, Vec<u8>)],
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
            let (_, bytes) = read_bytes(repo, name, None)?;
            hash_field(&mut digest, name.as_bytes());
            hash_field(&mut digest, &bytes);
        }
    }
    for (label, bytes) in configured_ignores {
        hash_field(&mut digest, label.as_bytes());
        hash_field(&mut digest, bytes);
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
fn configured_ignores(repo: &Path) -> Result<Vec<(&'static str, Vec<u8>)>, SnapshotError> {
    let mut inputs = Vec::new();
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
            let mut file = match fs::File::open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(ignore_error(label, &path, error)),
            };
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(|error| ignore_error(label, &path, error))?;
            // Preserve regular-file digests; an empty device such as /dev/null
            // contributes nothing, just like an absent configured file.
            if !bytes.is_empty()
                || file
                    .metadata()
                    .map_err(|error| ignore_error(label, &path, error))?
                    .is_file()
            {
                inputs.push((label, bytes));
            }
        }
    }
    Ok(inputs)
}

fn ignore_error(label: &str, path: &Path, error: std::io::Error) -> SnapshotError {
    SnapshotError::new(
        "snapshot-io",
        format!("{label} {}: {error}", path.display()),
    )
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
