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
            let config = model_config(&manifest.policy.paths, captured.entries(), &read_inventory)?;
            let inventory = warrant_inventory::build(&root, view, &manifest, &config)
                .map_err(inventory_error)?;
            let read_analysis = |path: &str| {
                captured
                    .read(path)
                    .map_err(|error| warrant_lang_ts::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            let mut reports = warrant_lang_ts::discover(&inventory.document)
                .iter()
                .map(|unit| warrant_lang_ts::analyze(unit, &inventory.document, &read_analysis))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(analysis_error)?;
            warrant_lang_ts::resolve(&mut reports, &inventory.document, &read_analysis)
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

fn model_config(
    configured: &[String],
    entries: &[warrant_core::nouns::InventoryEntry],
    read: warrant_inventory::Reader<'_>,
) -> std::result::Result<warrant_inventory::BuildConfig, warrant_snapshot::SnapshotError> {
    use warrant_core::policy::{ContractBody, PolicySource};
    let failure = |code: &str, reason: String| warrant_snapshot::SnapshotError {
        document: error_document(code, reason),
    };
    let mut paths = std::collections::BTreeSet::new();
    for pattern in configured {
        if Path::new(pattern).is_absolute()
            || Path::new(pattern)
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(failure(
                "policy-path",
                "policy paths must stay inside the snapshot".into(),
            ));
        }
        if let Some(directory) = pattern.strip_suffix("/*.yaml") {
            paths.extend(
                entries
                    .iter()
                    .filter(|entry| {
                        Path::new(&entry.path).parent() == Some(Path::new(directory))
                            && entry.path.ends_with(".yaml")
                    })
                    .map(|entry| entry.path.clone()),
            );
        } else if pattern.contains('*') {
            return Err(failure(
                "policy-path",
                format!("unsupported policy path pattern `{pattern}`"),
            ));
        } else {
            paths.insert(pattern.clone());
        }
    }
    let sources = paths
        .into_iter()
        .map(|path| {
            let bytes = read(&path).map_err(|error| failure(&error.code, error.reason))?;
            let yaml = String::from_utf8(bytes)
                .map_err(|_| failure("policy-encoding", format!("{path}: invalid UTF-8")))?;
            Ok((path, yaml))
        })
        .collect::<std::result::Result<Vec<_>, warrant_snapshot::SnapshotError>>()?;
    let sources: Vec<_> = sources
        .iter()
        .map(|(path, yaml)| PolicySource::new(path, yaml))
        .collect();
    let policy = warrant_core::policy::compile(&sources)
        .map_err(|error| failure(error.code(), error.to_string()))?;
    Ok(warrant_inventory::BuildConfig {
        modules: policy
            .contracts
            .into_iter()
            .filter_map(|contract| match contract.body {
                ContractBody::Module(module) => Some(warrant_inventory::ModuleSelector {
                    id: contract.id,
                    files: module.files.into_iter().collect(),
                }),
                _ => None,
            })
            .collect(),
        ..warrant_inventory::BuildConfig::default()
    })
}

fn model_rows(
    path: &Path,
) -> warrant_model::Result<BTreeMap<&'static str, warrant_model::query::QueryResult>> {
    use warrant_model::query::{QueryLimits, QueryStore};
    let store = QueryStore::open(path)?;
    let mut rows = BTreeMap::from([
        (
            "edges",
            store.sql(
                "SELECT files.path, edges.kind, edges.type_only, edges.unresolved_reason,
                    edges.resolved, target.path AS to_file, symbols.export_name AS to_symbol, edges.to_external
             FROM edges JOIN files ON files.id = edges.from_file
             LEFT JOIN files AS target ON target.id = edges.to_file
             LEFT JOIN symbols ON symbols.id = edges.to_symbol ORDER BY edges.id",
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
    ]);
    for (name, sql) in [
        (
            "module_edges",
            "SELECT source.name AS from_module, target.name AS to_module, edge_count, type_only_count FROM module_edges JOIN modules AS source ON source.id = from_module JOIN modules AS target ON target.id = to_module ORDER BY source.name, target.name",
        ),
        (
            "symbol_consumers",
            "SELECT files.path, symbols.export_name, consumer.path AS consumer_file, via_reexport_chain FROM symbol_consumers JOIN symbols ON symbols.id = symbol_id JOIN files ON files.id = symbols.file_id JOIN files AS consumer ON consumer.id = consumer_file ORDER BY files.path, symbols.export_name, consumer.path",
        ),
        (
            "module_cycles",
            "SELECT cycle_id, modules.name AS module, position FROM module_cycles JOIN modules ON modules.id = module_id ORDER BY cycle_id, position",
        ),
        (
            "observed_interfaces",
            "SELECT DISTINCT modules.name AS module, files.path, symbols.export_name, 'observed' AS basis FROM symbol_consumers JOIN symbols ON symbols.id = symbol_id JOIN files ON files.id = symbols.file_id JOIN modules ON modules.id = files.module_id JOIN files AS consumer ON consumer.id = consumer_file WHERE symbols.exported = 1 AND (consumer.module_id IS NULL OR consumer.module_id != files.module_id) ORDER BY modules.name, files.path, symbols.export_name",
        ),
        (
            "analysis_inputs",
            "SELECT COUNT(*) AS total_edges, COALESCE(SUM(NOT resolved), 0) AS unresolved_edges FROM edges",
        ),
    ] {
        rows.insert(name, store.sql(sql, QueryLimits::default())?);
    }
    Ok(rows)
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
