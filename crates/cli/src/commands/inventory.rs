use clap::Args as ClapArgs;
use warrant_core::nouns::SnapshotKind;

use crate::{
    cache, cancel, cli::Format, error::CommandError, manifest::load_manifest, output, repository,
};

use super::page;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Maximum number of inventory entries returned.
    #[arg(long, value_parser = page::parse_limit)]
    limit: Option<usize>,
    /// Zero-based cursor returned by a previous invocation.
    #[arg(long, default_value_t = 0)]
    cursor: usize,
}

pub fn run(args: Args, format: Option<Format>) -> crate::error::Result<()> {
    let root = repository::root()?;
    let manifest = load_manifest(&root)?;
    let (snapshot, built) = warrant_snapshot::capture(
        &root,
        SnapshotKind::Worktree,
        None,
        &manifest.snapshot,
        |captured| {
            cancel::check().map_err(|error| warrant_snapshot::SnapshotError {
                document: error.document.clone(),
            })?;
            let read = |path: &str| {
                captured
                    .read(path)
                    .map_err(|error| warrant_inventory::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            warrant_inventory::build(
                &root,
                warrant_inventory::CapturedSnapshot {
                    manifest: captured.manifest(),
                    entries: captured.entries(),
                    read: &read,
                },
                &manifest,
                &warrant_inventory::BuildConfig::default(),
            )
            .map_err(inventory_error)
        },
    )
    .map_err(snapshot_error)?;
    cancel::check()?;
    let bytes = serde_json::to_vec(&built.document)
        .map_err(|error| CommandError::internal(format!("could not encode inventory: {error}")))?;
    let analysis_key = built
        .digest
        .strip_prefix("sha256:")
        .unwrap_or(&built.digest);
    let path = cache::artifact_path(
        &snapshot.repo,
        &snapshot.tree,
        analysis_key,
        "inventory.json",
    );
    cache::write_atomic(&path, &bytes)?;
    let mut document = built.document;
    let page = page::bounds(
        document.entries.len(),
        args.limit.unwrap_or(usize::MAX),
        args.cursor,
    )?;
    document.entries = document.entries[page.range].to_vec();
    document.truncated = page.truncated;
    document.total = page.total;
    document.next_cursor = page.next_cursor;
    output::document(&document, format)
}

/// A refused snapshot read keeps the snapshot's code, so `snapshot-changed` still makes
/// capture retake the snapshot and a second change still ends in `snapshot-unstable`.
fn inventory_error(error: warrant_inventory::InventoryError) -> warrant_snapshot::SnapshotError {
    let document = match error {
        // The snapshot's reasons already name the path; add it only when missing.
        warrant_inventory::InventoryError::Read { path, code, reason } if reason == path => {
            error_document(&code, reason)
        }
        warrant_inventory::InventoryError::Read { path, code, reason } => {
            error_document(&code, format!("{path}: {reason}"))
        }
        error => error_document("inventory", error.to_string()),
    };
    warrant_snapshot::SnapshotError { document }
}

fn error_document(code: &str, reason: String) -> warrant_core::nouns::ErrorDocument {
    warrant_core::nouns::ErrorDocument {
        schema_version: "warrant.error/1".into(),
        code: code.into(),
        reason,
        locations: Vec::new(),
        next_diagnostic: None,
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
