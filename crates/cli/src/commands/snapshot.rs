use clap::{ArgGroup, Args as ClapArgs};
use warrant_core::{
    manifest::SnapshotConfig,
    nouns::{Capture, SnapshotKind},
};

use crate::{
    cache, cancel,
    cli::Format,
    error::CommandError,
    manifest::{load_snapshot_manifest, resolve_commit},
    output, repository,
};

// Spec 4.1: the worktree is the default when no kind is given; kinds stay exclusive.
#[derive(Debug, ClapArgs)]
#[command(group(ArgGroup::new("source").required(false).multiple(false)))]
pub struct Args {
    #[arg(long, group = "source")]
    worktree: bool,
    #[arg(long, group = "source")]
    index: bool,
    #[arg(long, value_name = "REV", group = "source")]
    commit: Option<String>,
    #[arg(long, value_name = "OID", group = "source")]
    tree: Option<String>,
}

pub fn run(args: Args, format: Option<Format>) -> crate::error::Result<()> {
    let root = repository::root()?;
    let cache_root = cache::root()?;
    let (kind, revision) = args.source();
    // A commit revision is resolved once, and the manifest and the capture both read that
    // commit id; a revision that names no commit ends the run as `missing-commit`.
    let commit = match (&kind, &revision) {
        (SnapshotKind::Commit, Some(revision)) => Some(resolve_commit(&root, revision)?),
        _ => None,
    };
    let governing = load_snapshot_manifest(&root, &kind, revision.as_deref(), commit.as_deref())?;
    let object = match kind {
        SnapshotKind::Commit => commit.as_deref(),
        _ => revision.as_deref(),
    };
    let (manifest, ()) =
        warrant_snapshot::capture(&root, kind, object, &governing.snapshot, |_| {
            cancel::check().map_err(|error| warrant_snapshot::SnapshotError {
                document: error.document.clone(),
            })?;
            Ok(())
        })
        .map_err(snapshot_error)?;
    cancel::check()?;
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| CommandError::internal(format!("could not encode snapshot: {error}")))?;
    let path = cache::artifact_path(
        &cache_root,
        &manifest.repo,
        &manifest.tree,
        &analysis_key(&manifest.capture, &governing.snapshot),
        "snapshot.json",
    );
    cache::write_atomic(&cache_root, &path, &bytes)?;
    output::document(&manifest, format)
}

/// The cache key names every capture input, so one tree captured under two limits
/// lands at two paths. The tree alone does not fix the limit: a worktree manifest can
/// be ignored, and a library caller can capture any object under any configuration.
/// The exhaustive destructure stops the build when `SnapshotConfig` gains an input.
fn analysis_key(capture: &Capture, config: &SnapshotConfig) -> String {
    let SnapshotConfig { max_file_bytes } = config;
    let ignore_inputs = capture
        .manifest_digest
        .as_deref()
        .unwrap_or("no-ignore-inputs")
        .replace(':', "-");
    format!(
        "snapshot-v1-{}-max-{max_file_bytes}-{ignore_inputs}",
        capture.kind
    )
}

impl Args {
    fn source(&self) -> (SnapshotKind, Option<String>) {
        if self.index {
            (SnapshotKind::Index, None)
        } else if let Some(revision) = &self.commit {
            (SnapshotKind::Commit, Some(revision.clone()))
        } else if let Some(tree) = &self.tree {
            (SnapshotKind::Tree, Some(tree.clone()))
        } else {
            (SnapshotKind::Worktree, None)
        }
    }
}

/// A recorded signal outranks this error: `main` reports cancellation first.
fn snapshot_error(error: warrant_snapshot::SnapshotError) -> Box<CommandError> {
    Box::new(CommandError {
        document: error.document,
        exit: 2,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn one_commit_under_two_limits_lands_at_two_paths() {
        let capture = Capture {
            kind: "commit".into(),
            manifest_digest: None,
        };
        let paths: Vec<_> = [40, 4096]
            .map(|max_file_bytes| {
                cache::artifact_path(
                    Path::new("/cache"),
                    "sha1:0000000000000000000000000000000000000000",
                    "sha1:1111111111111111111111111111111111111111",
                    &analysis_key(&capture, &SnapshotConfig { max_file_bytes }),
                    "snapshot.json",
                )
            })
            .into();
        assert_ne!(paths[0], paths[1]);
    }
}
