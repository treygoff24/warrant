//! Locates the repository once per command and runs the few Git reads the CLI owns.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use crate::error::CommandError;

/// Spec 4.2: a snapshot is a Git tree, so a directory outside Git has no identity.
const NON_REPOSITORY: &str = "Warrant needs a Git repository with history: a snapshot is a Git tree, and a directory without Git has no identity a receipt can name";

/// The working tree root that contains the current directory. Every command resolves
/// it here once, so a nested working directory cannot change the manifest or the paths.
pub fn root() -> crate::error::Result<PathBuf> {
    let output = git_in(Path::new("."), &["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Err(CommandError::evaluation(
            "non-repository",
            NON_REPOSITORY,
            None,
        ));
    }
    let mut bytes = output.stdout;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        CommandError::evaluation(
            "repository-io",
            "the repository path is not valid UTF-8",
            None,
        )
    })?;
    if text.is_empty() {
        return Err(CommandError::evaluation(
            "non-repository",
            NON_REPOSITORY,
            None,
        ));
    }
    Ok(PathBuf::from(text))
}

/// Run Git in `directory` with the snapshot crate's discipline: no optional locks, no
/// replace objects, and no inherited repository redirection from a surrounding hook.
pub fn git_in(directory: &Path, args: &[&str]) -> crate::error::Result<Output> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(variable);
    }
    command.output().map_err(|error| {
        CommandError::evaluation("repository-io", format!("could not run git: {error}"), None)
    })
}
