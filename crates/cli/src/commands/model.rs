/// Whether `warrant capabilities` reports this command as implemented.
pub const IMPLEMENTED: bool = true;

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use clap::Args as ClapArgs;
use warrant_core::nouns::{ErrorDocument, SnapshotKind};

use crate::{cache, cancel, error::CommandError, manifest::load_manifest, output, repository};

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Print the TypeScript integration's current analysis claims.
    #[arg(long)]
    capabilities: bool,
    /// Print persisted edge and unsupported rows (at most 1,000 rows and 1 MiB per table).
    #[arg(long, conflicts_with = "capabilities")]
    rows: bool,
}

pub fn run(args: Args, format: Option<crate::cli::Format>) -> crate::error::Result<()> {
    if args.capabilities {
        return output::document(&warrant_lang_ts::capabilities(), format);
    }

    let root = repository::root()?;
    let cache_root = cache::root()?;
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
            let read_inventory = |path: &str| {
                captured
                    .read(path)
                    .map_err(|error| warrant_inventory::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            let resolve = |path: &str| {
                captured
                    .resolve(path)
                    .map_err(|error| warrant_inventory::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            let view = warrant_inventory::CapturedSnapshot {
                manifest: captured.manifest(),
                entries: captured.entries(),
                read: &read_inventory,
                resolve: &resolve,
                untracked: captured.untracked(),
            };
            let inventory = warrant_inventory::build(
                &root,
                view,
                &manifest,
                &warrant_inventory::BuildConfig::default(),
            )
            .map_err(inventory_error)?;
            let read_analysis = |path: &str| {
                captured
                    .read(path)
                    .map_err(|error| warrant_lang_ts::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            let reports = warrant_lang_ts::discover(&inventory.document)
                .iter()
                .map(|unit| warrant_lang_ts::analyze(unit, &inventory.document, &read_analysis))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(analysis_error)?;
            Ok((inventory, reports))
        },
    )
    .map_err(snapshot_error)?;
    let (inventory, reports) = built;
    cancel::check()?;

    let analysis_key = inventory
        .digest
        .strip_prefix("sha256:")
        .unwrap_or(&inventory.digest);
    let path = cache::artifact_path(
        &cache_root,
        &snapshot.repo,
        &snapshot.tree,
        analysis_key,
        "model.sqlite",
    );
    publish_model(
        &path,
        &snapshot,
        &inventory.digest,
        warrant_lang_ts::module_rows(&inventory.document),
        reports,
        &root,
        &cache_root,
    )?;
    if args.rows {
        let rows = model_rows(&path).map_err(|error| model_error(error, &root, &cache_root))?;
        output::document(&rows, format)
    } else {
        output::document(&warrant_lang_ts::capabilities(), format)
    }
}

fn model_rows(
    path: &Path,
) -> warrant_model::Result<BTreeMap<&'static str, warrant_model::query::QueryResult>> {
    use warrant_model::query::{QueryLimits, QueryStore};
    let store = QueryStore::open(path)?;
    Ok(BTreeMap::from([
        (
            "edges",
            store.sql(
                "SELECT files.path, edges.kind, edges.type_only, edges.unresolved_reason,
                    edges.resolved, edges.to_file, edges.to_symbol, edges.to_external
             FROM edges JOIN files ON files.id = edges.from_file ORDER BY edges.id",
                QueryLimits::default(),
            )?,
        ),
        (
            "unsupported",
            store.sql(
                "SELECT files.path, unsupported.construct, unsupported.reason
             FROM unsupported JOIN files ON files.id = unsupported.file_id ORDER BY unsupported.id",
                QueryLimits::default(),
            )?,
        ),
    ]))
}

fn publish_model(
    path: &Path,
    snapshot: &warrant_core::nouns::SnapshotManifest,
    inventory_digest: &str,
    modules: Vec<warrant_model::ModuleRow>,
    reports: Vec<warrant_model::IntegrationReport>,
    root: &Path,
    cache_root: &Path,
) -> crate::error::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::internal("model cache path has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| {
        CommandError::internal(format!("could not create model cache: {error}"))
    })?;
    let temporary_directory = reserve_temporary(parent).map_err(|error| {
        CommandError::internal(format!("could not reserve model temporary: {error}"))
    })?;
    let temporary = temporary_directory.join("model.sqlite");

    let build = (|| -> warrant_model::Result<String> {
        let mut builder = warrant_model::ModelBuilder::create(&temporary)?;
        builder.write_meta("snapshot", &serde_json::to_string(snapshot)?)?;
        builder.write_meta("inventory_digest", inventory_digest)?;
        builder.write_capability_report(&warrant_model::CapabilityReportRow {
            integration: "lang-ts".into(),
            json: serde_json::to_value(warrant_lang_ts::capabilities())?,
        })?;
        for module in modules {
            builder.write_module(&module)?;
        }
        for report in reports {
            builder.write_report(&report)?;
        }
        builder.finish()
    })();
    if let Err(error) = build {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(model_error(error, root, cache_root));
    }
    if let Err(error) = cancel::check() {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error);
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_dir_all(&temporary_directory);
        CommandError::internal(format!("could not publish model cache: {error}"))
    })?;
    let _ = fs::remove_dir(&temporary_directory);
    Ok(())
}

fn reserve_temporary(parent: &Path) -> io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..128 {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".model.sqlite.{}.{sequence}.tmp",
            std::process::id()
        ));
        // Reserve a directory atomically: ModelBuilder requires a nonexistent database file.
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "model temporary retry limit reached",
    ))
}

fn model_error(
    error: warrant_model::ModelError,
    root: &Path,
    cache_root: &Path,
) -> Box<CommandError> {
    let reason = error
        .to_string()
        .replace(root.to_string_lossy().as_ref(), ".")
        .replace(cache_root.to_string_lossy().as_ref(), "<cache>");
    CommandError::internal(format!("could not build model: {reason}"))
}

fn analysis_error(error: warrant_lang_ts::AnalysisError) -> warrant_snapshot::SnapshotError {
    let document = match error {
        warrant_lang_ts::AnalysisError::Read { path, source } => ErrorDocument {
            schema_version: "warrant.error/1".into(),
            code: source.code,
            reason: format!("{path}: {}", source.reason),
            locations: Vec::new(),
            next_diagnostic: None,
        },
        error => error_document("typescript-analysis", error.to_string()),
    };
    warrant_snapshot::SnapshotError { document }
}

fn inventory_error(error: warrant_inventory::InventoryError) -> warrant_snapshot::SnapshotError {
    let document = match error {
        warrant_inventory::InventoryError::Read { path, code, reason } => ErrorDocument {
            schema_version: "warrant.error/1".into(),
            code,
            reason: format!("{path}: {reason}"),
            locations: Vec::new(),
            next_diagnostic: None,
        },
        error => error_document(error.code(), error.to_string()),
    };
    warrant_snapshot::SnapshotError { document }
}

fn error_document(code: &str, reason: String) -> ErrorDocument {
    ErrorDocument {
        schema_version: "warrant.error/1".into(),
        code: code.into(),
        reason,
        locations: Vec::new(),
        next_diagnostic: None,
    }
}

fn snapshot_error(error: warrant_snapshot::SnapshotError) -> Box<CommandError> {
    Box::new(CommandError {
        document: error.document,
        exit: 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use warrant_core::nouns::{Capture, SnapshotExclusions, SnapshotManifest};

    #[test]
    fn publish_model_survives_a_leaked_temporary() {
        let directory = tempfile::tempdir().expect("cache directory");
        let parent = directory.path();
        let stale = parent.join(format!(".model.sqlite.{}.tmp", std::process::id()));
        fs::write(&stale, b"previous interrupted build").expect("plant stale temporary");
        let collision = parent.join(format!(".model.sqlite.{}.0.tmp", std::process::id()));
        fs::write(&collision, b"reserved by an earlier process")
            .expect("plant candidate collision");
        let path = parent.join("model.sqlite");
        let snapshot = SnapshotManifest {
            schema_version: "warrant.snapshot/1".into(),
            repo: "sha1:repo".into(),
            kind: SnapshotKind::Worktree,
            tree: "sha1:tree".into(),
            object_format: "sha1".into(),
            commit: None,
            capture: Capture {
                kind: "fixture".into(),
                manifest_digest: None,
            },
            excluded: SnapshotExclusions::default(),
            taken_at: "fixture".into(),
        };
        publish_model(
            &path,
            &snapshot,
            "sha256:inventory",
            Vec::new(),
            Vec::new(),
            parent,
            parent,
        )
        .expect("publish despite stale temporary");
        assert!(
            fs::read(&path)
                .expect("published model")
                .starts_with(b"SQLite format 3\0")
        );
        assert_eq!(
            fs::read(&stale).expect("preserve stale temporary"),
            b"previous interrupted build"
        );
        assert_eq!(
            fs::read(&collision).expect("preserve collision"),
            b"reserved by an earlier process"
        );
        assert_eq!(fs::read_dir(parent).expect("cache contents").count(), 3);
    }
}
