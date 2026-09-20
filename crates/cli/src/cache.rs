use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{cancel, error::CommandError};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub fn artifact_path(repo: &str, tree: &str, analysis_key: &str, name: &str) -> PathBuf {
    let cache = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    cache
        .join("warrant")
        .join(repo.replace(':', "-"))
        .join("snapshots")
        .join(tree.rsplit(':').next().unwrap_or(tree))
        .join(analysis_key)
        .join(name)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> crate::error::Result<()> {
    cancel::check()?;
    let parent = path.parent().ok_or_else(|| {
        CommandError::internal(format!("cache path has no parent: {}", path.display()))
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

fn io_error(path: &Path, error: std::io::Error) -> Box<CommandError> {
    CommandError::internal(format!("{}: {error}", path.display()))
}
