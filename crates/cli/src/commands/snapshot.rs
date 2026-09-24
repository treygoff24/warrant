use clap::{ArgGroup, Args as ClapArgs};
use warrant_core::nouns::SnapshotKind;

use crate::{
    cache, cancel, cli::Format, error::CommandError, manifest::load_manifest, output, repository,
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
    let manifest = load_manifest(&root)?;
    let (kind, revision) = args.source();
    let (manifest, ()) =
        warrant_snapshot::capture(&root, kind, revision.as_deref(), &manifest.snapshot, |_| {
            cancel::check().map_err(|error| warrant_snapshot::SnapshotError {
                document: error.document.clone(),
            })?;
            Ok(())
        })
        .map_err(snapshot_error)?;
    cancel::check()?;
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| CommandError::internal(format!("could not encode snapshot: {error}")))?;
    let input_digest = manifest
        .capture
        .manifest_digest
        .as_deref()
        .unwrap_or("no-capture-inputs")
        .replace(':', "-");
    let analysis_key = format!("snapshot-v1-{}-{input_digest}", manifest.capture.kind);
    let path = cache::artifact_path(
        &manifest.repo,
        &manifest.tree,
        &analysis_key,
        "snapshot.json",
    );
    cache::write_atomic(&path, &bytes)?;
    output::document(&manifest, format)
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

fn snapshot_error(error: warrant_snapshot::SnapshotError) -> Box<CommandError> {
    if error.document.code == "cancelled"
        && let Err(cancelled) = cancel::check()
    {
        return cancelled;
    }
    Box::new(CommandError {
        document: error.document,
        exit: 2,
    })
}
