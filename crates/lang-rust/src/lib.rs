//! Cargo package facts only; no Rust source resolution or macro analysis.

use std::{collections::BTreeMap, path::Path, process::Command};

use cargo_metadata::{CargoOpt, Metadata, MetadataCommand};
use warrant_core::nouns::{CapabilityReport, InventoryEntry, UnsupportedCapability};
use warrant_model::{
    CapabilityReportRow, EdgeRow, EntrypointRow, FileRow, IntegrationReport, ModelError, Result,
    UnitRow, UnsupportedRow,
};

const CARGO_VERSION: &str = "1.98.1";
const BASIS: &str = "native:cargo-metadata";
const UNSUPPORTED: &[&str] = &[
    "rust-internal-references",
    "rust-internal-visibility",
    "rust-use-graph",
    "rust-macro-expansion",
];

/// Explicit Cargo resolution inputs. An empty target is refused.
#[derive(Clone, Debug)]
pub struct Selection {
    pub target: String,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub all_features: bool,
}

/// Metadata captured with the pinned Cargo and the recorded selection.
/// Fields are private so unqualified metadata cannot acquire native authority.
#[derive(Debug)]
pub struct CargoInput {
    metadata: Metadata,
    selection: Selection,
}

/// Capture a materialized snapshot with Cargo 1.98.1, format 1, locked and offline.
/// The caller must supply the same snapshot tree used to build the inventory.
/// Cargo reads configuration and manifests; this does not build or run targets.
pub fn capture(root: &Path, selection: &Selection) -> Result<CargoInput> {
    if selection.target.trim().is_empty() {
        return Err(invalid("Cargo target selection is required"));
    }
    let root = root.canonicalize()?;
    let version = Command::new("cargo")
        .arg("--version")
        .env("RUSTUP_TOOLCHAIN", CARGO_VERSION)
        .current_dir(&root)
        .output()?;
    if !version.status.success()
        || std::str::from_utf8(&version.stdout)
            .ok()
            .and_then(|text| text.split_whitespace().nth(1))
            != Some(CARGO_VERSION)
    {
        return Err(invalid("pinned Cargo 1.98.1 is unavailable or mismatched"));
    }
    let mut selection = selection.clone();
    selection.features.sort();
    selection.features.dedup();
    let mut command = MetadataCommand::new();
    command
        .cargo_path("cargo")
        .current_dir(&root)
        .manifest_path(root.join("Cargo.toml"))
        .env("RUSTUP_TOOLCHAIN", CARGO_VERSION)
        .other_options(vec![
            "--locked".into(),
            "--offline".into(),
            "--filter-platform".into(),
            selection.target.clone(),
        ]);
    if selection.no_default_features {
        command.features(CargoOpt::NoDefaultFeatures);
    }
    if selection.all_features {
        command.features(CargoOpt::AllFeatures);
    }
    command.features(CargoOpt::SomeFeatures(selection.features.clone()));
    let metadata = command.exec().map_err(|_| {
        // Cargo stderr may contain local paths or credentials in registry URLs.
        invalid(
            "locked offline Cargo metadata failed; check the lockfile, dependencies and selection",
        )
    })?;
    if metadata.workspace_root.as_std_path() != root {
        return Err(invalid("capture must start at the Cargo workspace root"));
    }
    Ok(CargoInput {
        metadata,
        selection,
    })
}

/// Native authority covers resolved package-dependency edges only.
pub fn capabilities() -> CapabilityReport {
    CapabilityReport {
        schema_version: "warrant.capabilities/1".into(),
        integration: "lang-rust".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        instruments: BTreeMap::from([
            ("cargo".into(), CARGO_VERSION.into()),
            ("cargo_metadata".into(), "0.23.1".into()),
        ]),
        resolution_oracle: Some("cargo metadata --format-version 1 --locked --offline --filter-platform <target>".into()),
        compiler_reference_instrument: None,
        resolution_authority: "native".into(),
        resolution_modes_qualified: Vec::new(),
        symbol_level: "none".into(),
        type_only_distinction: false,
        supports: vec!["package-dependency".into()],
        unsupported: UNSUPPORTED.iter().map(|construct| UnsupportedCapability {
            construct: (*construct).into(),
            treatment: "not-observed".into(),
        }).collect(),
        limits: "Native build-system authority only for resolved package-dependency edges under the recorded Cargo target and features. Targets are manifest observations, not evidence of execution. No Rust-internal references, visibility, use graph or macro expansion is analyzed. Cargo may unify features across workspace members and dependency kinds; this is not a per-target compilation graph.".into(),
    }
}

/// Discover one unit per workspace package, independent of third-party packages.
pub fn discover(input: &CargoInput) -> Result<Vec<UnitRow>> {
    let mut packages = input.metadata.workspace_packages();
    packages.sort_by(|a, b| a.manifest_path.cmp(&b.manifest_path));
    packages
        .iter()
        .enumerate()
        .map(|(index, package)| {
            let path = relative(input, package.manifest_path.as_std_path())?;
            let root = Path::new(&path)
                .parent()
                .and_then(Path::to_str)
                .filter(|path| !path.is_empty())
                .unwrap_or(".")
                .to_owned();
            Ok(UnitRow {
                // TypeScript currently allocates positive unit and observation ids.
                id: negative_id(index)?,
                integration: "lang-rust".into(),
                root,
                config_path: Some(path),
                kind: "cargo-package".into(),
            })
        })
        .collect()
}

/// Convert captured Cargo facts and inventory identities to shared model reports.
/// The caller writes inventory module rows before these reports, as for lang-ts.
/// Manifest files anchor package edges; source files carry unsupported internals.
pub fn analyze(input: &CargoInput, inventory: &[InventoryEntry]) -> Result<Vec<IntegrationReport>> {
    let resolve = input
        .metadata
        .resolve
        .as_ref()
        .ok_or_else(|| invalid("Cargo metadata has no resolve graph"))?;
    let mut packages = input.metadata.workspace_packages();
    packages.sort_by(|a, b| a.manifest_path.cmp(&b.manifest_path));
    let units = discover(input)?;
    let mut modules = inventory
        .iter()
        .filter_map(|entry| entry.module.as_deref())
        .collect::<Vec<_>>();
    modules.sort();
    modules.dedup();
    let mut files = BTreeMap::new();
    for (index, entry) in inventory.iter().enumerate() {
        if files.insert(entry.path.as_str(), (index, entry)).is_some() {
            return Err(invalid("duplicate inventory path"));
        }
    }
    let file_row = |path: &str, unit_id: i64| -> Result<FileRow> {
        let (index, entry) = files
            .get(path)
            .ok_or_else(|| invalid(format!("Cargo file is absent from inventory: {path}")))?;
        if entry.unread.is_some() {
            return Err(invalid(format!("Cargo file is unread: {path}")));
        }
        Ok(FileRow {
            id: -negative_id(*index)?,
            path: path.into(),
            blob: entry
                .blob
                .clone()
                .ok_or_else(|| invalid(format!("Cargo file has no captured blob: {path}")))?,
            class: entry.class.as_str().into(),
            language: entry.language.clone(),
            unit_id,
            module_id: entry.module.as_deref().map(|name| {
                modules
                    .binary_search(&name)
                    .map(|i| (i + 1) as i64)
                    .expect("collected module")
            }),
        })
    };
    let mut capabilities = capabilities();
    capabilities.limits.push_str(&format!(
        " Selection: target={}; features={:?}; no-default-features={}; all-features={}.",
        input.selection.target,
        input.selection.features,
        input.selection.no_default_features,
        input.selection.all_features,
    ));
    let capability_report = CapabilityReportRow {
        integration: "lang-rust".into(),
        json: serde_json::to_value(capabilities)?,
    };
    let mut reports = Vec::new();
    let mut edge_index = 0;
    let mut entrypoint_index = 0;
    let mut unsupported_index = 0;
    for (package, unit) in packages.iter().zip(&units) {
        let manifest = relative(input, package.manifest_path.as_std_path())?;
        let manifest_row = file_row(&manifest, unit.id)?;
        let node = resolve
            .nodes
            .iter()
            .find(|node| node.id == package.id)
            .ok_or_else(|| invalid("workspace package is absent from Cargo resolve graph"))?;
        let mut report = IntegrationReport {
            unit: unit.clone(),
            files: vec![manifest_row.clone()],
            symbols: Vec::new(),
            edges: Vec::new(),
            references: Vec::new(),
            entrypoints: Vec::new(),
            effects: Vec::new(),
            unsupported: Vec::new(),
            capability_report: capability_report.clone(),
        };
        let target_paths = package
            .targets
            .iter()
            .map(|target| relative(input, target.src_path.as_std_path()))
            .collect::<Result<Vec<_>>>()?;
        for entry in inventory
            .iter()
            .filter(|entry| entry.path.ends_with(".rs") || target_paths.contains(&entry.path))
        {
            let owner = units
                .iter()
                .filter(|candidate| {
                    candidate.root == "."
                        || entry
                            .path
                            .strip_prefix(&candidate.root)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
                .max_by_key(|candidate| candidate.root.len());
            if owner.map(|owner| owner.id) != Some(unit.id) {
                continue;
            }
            let file = file_row(&entry.path, unit.id)?;
            for construct in UNSUPPORTED {
                report.unsupported.push(UnsupportedRow {
                    id: negative_id(unsupported_index)?,
                    file_id: file.id,
                    construct: (*construct).into(),
                    span_start: 0,
                    span_end: 0,
                    reason: "Cargo metadata supplies no Rust-internal analysis".into(),
                });
                unsupported_index += 1;
            }
            report.files.push(file);
        }
        let mut targets = package.targets.iter().collect::<Vec<_>>();
        targets.sort_by(|a, b| (&a.src_path, &a.name).cmp(&(&b.src_path, &b.name)));
        for target in targets {
            if !target.required_features.iter().all(|feature| {
                node.features
                    .iter()
                    .any(|enabled| enabled.as_str() == feature)
            }) {
                continue;
            }
            let path = relative(input, target.src_path.as_std_path())?;
            let file = file_row(&path, unit.id)?;
            if !report.files.iter().any(|row| row.id == file.id) {
                return Err(invalid(format!(
                    "Cargo target is outside its package source inventory: {path}"
                )));
            }
            for kind in &target.kind {
                report.entrypoints.push(EntrypointRow {
                    id: negative_id(entrypoint_index)?,
                    file_id: file.id,
                    symbol_id: None,
                    kind: format!("cargo-target:{kind}"),
                    basis: "observed".into(),
                    declared_by: Some(target.name.clone()),
                });
                entrypoint_index += 1;
            }
        }
        let mut dependencies = node.deps.iter().collect::<Vec<_>>();
        dependencies.sort_by(|a, b| (&a.name, &a.pkg).cmp(&(&b.name, &b.pkg)));
        for dependency in dependencies {
            let target = input
                .metadata
                .packages
                .iter()
                .find(|p| p.id == dependency.pkg)
                .ok_or_else(|| invalid("resolved Cargo dependency is absent from packages"))?;
            let (to_file, to_external) =
                if let Some(index) = packages.iter().position(|p| p.id == target.id) {
                    let path = relative(input, target.manifest_path.as_std_path())?;
                    (Some(file_row(&path, units[index].id)?.id), None)
                } else {
                    let identity = if target.source.is_some() {
                        target.id.to_string()
                    } else {
                        // Preserve path-package identity without binding it to a checkout location.
                        let path = relative(input, target.manifest_path.as_std_path())?;
                        format!("path:{path}#{}@{}", target.name, target.version)
                    };
                    (None, Some(identity))
                };
            report.edges.push(EdgeRow {
                id: negative_id(edge_index)?,
                kind: "package-dependency".into(),
                from_file: manifest_row.id,
                from_symbol: None,
                to_file,
                to_symbol: None,
                to_external,
                resolved: true,
                unresolved_reason: None,
                type_only: false,
                span_start: 0,
                span_end: 0,
                basis: BASIS.into(),
            });
            edge_index += 1;
        }
        reports.push(report);
    }
    Ok(reports)
}

fn relative(input: &CargoInput, path: &Path) -> Result<String> {
    let path = path
        .strip_prefix(input.metadata.workspace_root.as_std_path())
        .map_err(|_| invalid("Cargo workspace file is outside the captured root"))?;
    if path
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(invalid(
            "Cargo workspace file is not a normalized relative path",
        ));
    }
    path.to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or_else(|| invalid("Cargo path is not UTF-8"))
}

fn negative_id(index: usize) -> Result<i64> {
    i64::try_from(index)
        .ok()
        .and_then(|id| id.checked_add(1))
        .and_then(i64::checked_neg)
        .ok_or_else(|| invalid("Cargo model exceeds SQLite ids"))
}

fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
