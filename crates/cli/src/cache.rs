use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{cancel, error::CommandError};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// The cache root: `$XDG_CACHE_HOME`, else `$HOME/.cache`. An empty or relative value
/// counts as unset (the XDG base-directory rule), and with neither there is no cache
/// location: a relative `.cache` would land in whatever directory the run started in.
/// Commands resolve it before capture, so a missing location fails before any work.
pub fn root() -> crate::error::Result<PathBuf> {
    let absolute = |name: &str| {
        env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    absolute("XDG_CACHE_HOME")
        .or_else(|| absolute("HOME").map(|home| home.join(".cache")))
        .ok_or_else(|| {
            CommandError::evaluation(
                "cache-location",
                "no cache location: neither XDG_CACHE_HOME nor HOME is set to an absolute path",
                Some("set XDG_CACHE_HOME or HOME to an absolute directory".into()),
            )
        })
}

pub fn artifact_path(
    cache: &Path,
    repo: &str,
    tree: &str,
    analysis_key: &str,
    name: &str,
) -> PathBuf {
    cache
        .join("warrant")
        .join(repo.replace(':', "-"))
        .join("snapshots")
        .join(tree.rsplit(':').next().unwrap_or(tree))
        .join(analysis_key)
        .join(name)
}

/// Write `bytes` to `path` below the cache root `cache` atomically. Error documents name
/// cache paths relative to the cache root, never by their absolute path.
pub fn write_atomic(cache: &Path, path: &Path, bytes: &[u8]) -> crate::error::Result<()> {
    cancel::check()?;
    let io_error = |at: &Path, error: std::io::Error| io_error(cache, at, error);
    let parent = path.parent().ok_or_else(|| {
        CommandError::internal(format!(
            "cache path has no parent: {}",
            relative(cache, path).display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), id));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error(&temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error(&temporary, error))?;
        file.sync_all()
            .map_err(|error| io_error(&temporary, error))?;
        cancel::check()?;
        fs::rename(&temporary, path).map_err(|error| io_error(path, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn io_error(cache: &Path, path: &Path, error: std::io::Error) -> Box<CommandError> {
    CommandError::internal(format!(
        "cache {}: {error}",
        relative(cache, path).display()
    ))
}

/// `path` below the cache root; the root itself reads as `.`.
fn relative<'a>(cache: &Path, path: &'a Path) -> &'a Path {
    match path.strip_prefix(cache) {
        Ok(relative) if relative.as_os_str().is_empty() => Path::new("."),
        Ok(relative) => relative,
        Err(_) => path,
    }
}
