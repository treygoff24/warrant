use crate::SnapshotError;
use std::{
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
};

pub(crate) fn run(
    repo: &Path,
    args: &[&str],
    index: Option<&Path>,
    mut input: Option<&[u8]>,
) -> Result<Vec<u8>, SnapshotError> {
    run_stream(
        repo,
        args,
        index,
        input.as_mut().map(|bytes| bytes as &mut dyn Read),
    )
}

pub(crate) fn run_stream(
    repo: &Path,
    args: &[&str],
    index: Option<&Path>,
    input: Option<&mut dyn Read>,
) -> Result<Vec<u8>, SnapshotError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // An explicit repository must not be redirected by a surrounding Git hook.
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
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command.spawn()?;
    let written = if let Some(input) = input {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SnapshotError::new("snapshot-io", "missing Git stdin"))?;
        io::copy(input, &mut stdin).map(|_| ())
    } else {
        Ok(())
    };
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(SnapshotError::new(
            "git-failed",
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }
    written?;
    Ok(output.stdout)
}

pub(crate) fn text(
    repo: &Path,
    args: &[&str],
    index: Option<&Path>,
) -> Result<String, SnapshotError> {
    let bytes = run(repo, args, index, None)?;
    Ok(String::from_utf8(bytes)
        .map_err(|_| SnapshotError::new("git-output", "non-UTF-8 Git output"))?
        .trim()
        .into())
}

pub(crate) fn check_index(repo: &Path) -> Result<(), SnapshotError> {
    if !run(repo, &["ls-files", "--unmerged", "-z"], None, None)?.is_empty() {
        return Err(SnapshotError::new(
            "unmerged-index",
            "resolve index conflicts before capture",
        ));
    }
    Ok(())
}
