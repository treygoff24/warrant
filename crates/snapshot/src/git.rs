use crate::SnapshotError;
use std::{
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
};

#[cfg(test)]
thread_local! {
    pub(crate) static CALLS: std::cell::RefCell<Option<Vec<Vec<String>>>> = const { std::cell::RefCell::new(None) };
}

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
    #[cfg(test)]
    CALLS.with_borrow_mut(|calls| {
        if let Some(calls) = calls {
            calls.push(args.iter().map(|arg| (*arg).to_owned()).collect());
        }
    });
    let mut command = Command::new("git");
    #[cfg(test)]
    crate::tests::neutralize_git_environment(&mut command);
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
    let stdin = child.stdin.take();
    // Read is not necessarily Send. Keep the borrowed input on this thread
    // while a scoped worker drains both output pipes and waits for Git.
    let (written, output) = std::thread::scope(|scope| {
        let output = scope.spawn(move || child.wait_with_output());
        let written = if let Some(input) = input {
            match stdin {
                Some(mut stdin) => io::copy(input, &mut stdin).map(|_| ()),
                None => Err(io::Error::other("missing Git stdin")),
            }
        } else {
            Ok(())
        };
        (written, output.join())
    });
    let output = output.map_err(|_| io::Error::other("Git output reader panicked"))??;
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
