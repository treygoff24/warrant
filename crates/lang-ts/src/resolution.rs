//! Resolve against the captured inventory, never the process filesystem.
use crate::{AnalysisError, Reader, Result};
use oxc_resolver::{
    FileMetadata, FileSystem, ResolveContext, ResolveError, ResolveOptions, ResolverGeneric,
    TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};
use warrant_core::nouns::InventoryDocument;
use warrant_model::IntegrationReport;

mod requests;

#[derive(Default)]
struct CapturedFiles(BTreeMap<PathBuf, Vec<u8>>);

impl FileSystem for CapturedFiles {
    fn new() -> Self {
        Self::default()
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        if self.0.contains_key(path) {
            return Ok(FileMetadata::new(true, false, false));
        }
        if self
            .0
            .range(path.to_path_buf()..)
            .next()
            .is_some_and(|(file, _)| file.starts_with(path))
        {
            return Ok(FileMetadata::new(false, true, false));
        }
        Err(io::ErrorKind::NotFound.into())
    }
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata(path)
    }
    fn read_link(&self, _: &Path) -> std::result::Result<PathBuf, ResolveError> {
        Err(io::Error::from(io::ErrorKind::InvalidInput).into())
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.metadata(path)?;
        Ok(path.to_owned())
    }
}

/// Resolve the syntax reports together so cross-unit targets use the same file and symbol ids.
/// Missing targets remain edges with reasons; callers persist every report before finishing the model.
pub fn resolve(
    reports: &mut [IntegrationReport],
    inventory: &InventoryDocument,
    read: Reader<'_>,
) -> Result<()> {
    let root = Path::new("/warrant-capture");
    let mut captured = CapturedFiles::default();
    for entry in &inventory.entries {
        if entry.blob.is_none() || entry.unread.is_some() {
            continue;
        }
        let bytes = if entry.path.ends_with(".json") || entry.path.ends_with(".jsonc") {
            read(&entry.path).map_err(|source| AnalysisError::Read {
                path: entry.path.clone(),
                source,
            })?
        } else {
            Vec::new()
        };
        captured.0.insert(root.join(&entry.path), bytes);
    }
    let packages: BTreeMap<_, serde_json::Value> = captured
        .0
        .iter()
        .filter(|(path, _)| path.file_name().is_some_and(|name| name == "package.json"))
        .filter_map(|(path, bytes)| {
            serde_json::from_slice(bytes)
                .ok()
                .map(|json| (path.clone(), json))
        })
        .collect();
    let resolver = ResolverGeneric::new_with_file_system(captured, ResolveOptions::default());
    let files: BTreeMap<_, _> = reports
        .iter()
        .flat_map(|report| report.files.iter())
        .map(|file| (root.join(&file.path), file.id))
        .collect();
    let exports: BTreeMap<_, _> = reports
        .iter()
        .flat_map(|report| report.symbols.iter())
        .filter_map(|symbol| {
            symbol
                .export_name
                .as_ref()
                .map(|name| ((symbol.file_id, name.clone()), symbol.id))
        })
        .collect();

    let mut names = BTreeMap::new();
    for report in reports.iter_mut() {
        let config_path = report
            .unit
            .config_path
            .as_ref()
            .filter(|_| report.unit.kind == "tsconfig");
        let base_options = ResolveOptions {
            tsconfig: config_path
                .map(|path| {
                    TsconfigDiscovery::Manual(TsconfigOptions {
                        config_file: root.join(path),
                        references: TsconfigReferences::Auto,
                    })
                })
                .or(Some(TsconfigDiscovery::Auto)),
            extensions: [
                ".ts", ".tsx", ".d.ts", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".json",
            ]
            .map(str::to_owned)
            .to_vec(),
            extension_alias: [
                (".js", vec![".ts", ".tsx", ".d.ts", ".js"]),
                (".mjs", vec![".mts", ".d.mts", ".mjs"]),
                (".cjs", vec![".cts", ".d.cts", ".cjs"]),
            ]
            .into_iter()
            .map(|(key, values)| (key.into(), values.into_iter().map(str::to_owned).collect()))
            .collect(),
            main_fields: vec!["types".into(), "typings".into(), "main".into()],
            builtin_modules: true,
            ..ResolveOptions::default()
        };
        // A separate config cache per unit prevents manual-tsconfig cache entries crossing units.
        let unit_resolver = resolver.clone_with_options(base_options.clone());
        unit_resolver.clear_cache();
        for file in &report.files {
            let absolute = root.join(&file.path);
            let source = read(&file.path).map_err(|source| AnalysisError::Read {
                path: file.path.clone(),
                source,
            })?;
            let requests = requests::collect(&file.path, &source);
            let config = unit_resolver.find_tsconfig(&absolute).map(|config| {
                config.map(|config| {
                    let mut config = (*config).clone();
                    // Keep paths_base intact: baseUrl still anchors explicit paths mappings in TS6+.
                    if !legacy_typescript(&absolute, &packages) {
                        config.compiler_options.base_url = None;
                    }
                    config
                })
            });
            for edge in report.edges.iter_mut().filter(|edge| {
                edge.from_file == file.id
                    && edge.unresolved_reason.as_deref() == Some("resolution-pending")
            }) {
                let Some(request) = requests.iter().find(|request| {
                    request.start == edge.span_start && request.end == edge.span_end
                }) else {
                    edge.unresolved_reason = Some("module-specifier-not-observed".into());
                    continue;
                };
                names.insert(edge.id, request.name.clone());
                let config = match &config {
                    Ok(config) => config.as_ref(),
                    Err(error) => {
                        edge.unresolved_reason = Some(reason(error, root));
                        continue;
                    }
                };
                let module = config
                    .and_then(|config| config.compiler_options.module.as_deref())
                    .unwrap_or("nodenext")
                    .to_ascii_lowercase();
                let require = edge.kind == "require"
                    || (edge.kind != "dynamic_import"
                        && (module == "commonjs"
                            || ((module == "nodenext"
                                || module.starts_with("node1")
                                || module == "node20")
                                && !is_esm(&absolute, &packages))));
                let mut options = base_options.clone();
                options.condition_names = vec![
                    "types".into(),
                    if require { "require" } else { "import" }.into(),
                ];
                if module.starts_with("node") || module == "commonjs" {
                    options.condition_names.push("node".into());
                }
                let edge_resolver = unit_resolver.clone_with_options(options);
                let result = edge_resolver.resolve_with_context(
                    absolute.parent().expect("captured file has parent"),
                    &request.specifier,
                    config,
                    &mut ResolveContext::default(),
                );
                match result {
                    Ok(target) => {
                        if let Some(id) = files.get(target.path()) {
                            edge.to_file = Some(*id);
                            edge.to_symbol = request
                                .name
                                .as_ref()
                                .and_then(|name| exports.get(&(*id, name.clone())))
                                .copied();
                            edge.resolved = true;
                            edge.unresolved_reason = None;
                        } else if target
                            .path()
                            .components()
                            .any(|part| part.as_os_str() == "node_modules")
                            && let Some(package) =
                                target.package_json().and_then(|package| package.name())
                        {
                            edge.to_external = Some(package.to_owned());
                            edge.resolved = true;
                            edge.unresolved_reason = None;
                        } else {
                            edge.unresolved_reason = Some("target-outside-analyzed-files".into());
                        }
                    }
                    Err(ResolveError::Builtin { resolved, .. }) => {
                        edge.to_external = Some(resolved);
                        edge.resolved = true;
                        edge.unresolved_reason = None;
                    }
                    Err(error) => edge.unresolved_reason = Some(reason(&error, root)),
                }
            }
        }
    }
    bind_symbols(reports, &names)?;
    Ok(())
}

fn reason(error: &ResolveError, root: &Path) -> String {
    match error {
        ResolveError::NotFound(_) => return "module-not-found".into(),
        ResolveError::PackagePathNotExported { .. } => return "package-path-not-exported".into(),
        _ => {}
    }
    format!(
        "resolution-failed: {}",
        error
            .to_string()
            .replace(root.to_str().expect("literal root"), ".")
    )
}

fn is_esm(path: &Path, packages: &BTreeMap<PathBuf, serde_json::Value>) -> bool {
    match path.extension().and_then(|value| value.to_str()) {
        Some("mts" | "mjs") => true,
        Some("cts" | "cjs") => false,
        _ => path
            .ancestors()
            .skip(1)
            .find_map(|directory| packages.get(&directory.join("package.json")))
            .is_some_and(|package| package["type"] == "module"),
    }
}

fn legacy_typescript(path: &Path, packages: &BTreeMap<PathBuf, serde_json::Value>) -> bool {
    for directory in path.ancestors().skip(1) {
        let installed = packages
            .get(&directory.join("node_modules/typescript/package.json"))
            .and_then(|package| package["version"].as_str());
        let declared = packages
            .get(&directory.join("package.json"))
            .and_then(|package| {
                package["devDependencies"]["typescript"]
                    .as_str()
                    .or_else(|| package["dependencies"]["typescript"].as_str())
            });
        if let Some(version) = installed.or(declared) {
            return version
                .trim_start_matches(['^', '~'])
                .split('.')
                .next()
                .and_then(|major| major.parse::<u64>().ok())
                .is_some_and(|major| major < 6);
        }
    }
    // The integration targets TS7; unknown versions never enable the removed fallback.
    false
}

fn bind_symbols(
    reports: &mut [IntegrationReport],
    names: &BTreeMap<i64, Option<String>>,
) -> Result<()> {
    use std::collections::BTreeSet;
    let mut exports: BTreeMap<_, _> = reports
        .iter()
        .flat_map(|report| &report.symbols)
        .filter_map(|symbol| {
            symbol
                .export_name
                .as_ref()
                .map(|name| ((symbol.file_id, name.clone()), symbol.id))
        })
        .collect();
    let explicit = exports.clone();
    let stars: Vec<_> = reports
        .iter()
        .enumerate()
        .flat_map(|(index, report)| {
            report
                .edges
                .iter()
                .filter(|edge| {
                    edge.kind == "reexport" && edge.from_symbol.is_none() && edge.to_file.is_some()
                })
                .map(move |edge| (index, edge.clone()))
        })
        .collect();
    let links: BTreeMap<_, _> = reports
        .iter()
        .flat_map(|report| &report.edges)
        .filter_map(|edge| edge.from_symbol.zip(edge.to_symbol))
        .collect();
    let mut candidates: BTreeMap<_, BTreeSet<i64>> = exports
        .iter()
        .map(|(key, id)| {
            let mut origin = *id;
            let mut visited = BTreeSet::new();
            while visited.insert(origin) {
                let Some(target) = links.get(&origin) else {
                    break;
                };
                origin = *target;
            }
            (key.clone(), BTreeSet::from([origin]))
        })
        .collect();
    // Union origin bindings to a fixed point before choosing exports. First-match-wins would
    // silently choose one binding from conflicting stars, and would depend on traversal order.
    loop {
        let previous = candidates.clone();
        for (_, star) in &stars {
            for ((file, name), origins) in &previous {
                let key = (star.from_file, name.clone());
                if Some(*file) == star.to_file && name != "default" && !explicit.contains_key(&key)
                {
                    candidates.entry(key).or_default().extend(origins);
                }
            }
        }
        if candidates == previous {
            break;
        }
    }
    for ((file, name), origins) in &candidates {
        if explicit.contains_key(&(*file, name.clone())) {
            continue;
        }
        let (index, star) = stars
            .iter()
            .find(|(_, edge)| edge.from_file == *file)
            .expect("star owner");
        let report = &mut reports[*index];
        if origins.len() != 1 {
            crate::push_unsupported(
                report,
                *file,
                oxc_span::Span::new(star.span_start as u32, star.span_end as u32),
                "star-reexport",
                "ambiguous-star-export",
            )?;
            continue;
        }
        let next = report
            .symbols
            .iter()
            .filter(|row| row.file_id == *file)
            .map(|row| row.id & 0xffff_ffff)
            .max()
            .unwrap_or(0)
            + 1;
        let id = crate::row_id(*file, next as usize)?;
        report.symbols.push(warrant_model::SymbolRow {
            id,
            file_id: *file,
            name: name.clone(),
            kind: "reexport".into(),
            exported: true,
            export_name: Some(name.clone()),
            visibility: "public".into(),
            span_start: star.span_start,
            span_end: star.span_end,
        });
        exports.insert((*file, name.clone()), id);
    }
    for (index, star) in &stars {
        let target = star.to_file.expect("resolved star");
        let report = &mut reports[*index];
        for ((file, name), symbol) in &exports {
            if *file != target
                || name == "default"
                || explicit.contains_key(&(star.from_file, name.clone()))
            {
                continue;
            }
            let Some(from) = exports.get(&(star.from_file, name.clone())) else {
                continue;
            };
            let mut edge = star.clone();
            edge.from_symbol = Some(*from);
            edge.to_symbol = Some(*symbol);
            if let Some(original) = report
                .edges
                .iter_mut()
                .find(|edge| edge.id == star.id && edge.from_symbol.is_none())
            {
                *original = edge;
            } else {
                let next = report
                    .edges
                    .iter()
                    .filter(|edge| edge.from_file == star.from_file)
                    .map(|edge| edge.id & 0xffff_ffff)
                    .max()
                    .unwrap_or(0)
                    + 1;
                edge.id = crate::row_id(star.from_file, next as usize)?;
                report.edges.push(edge);
            }
        }
    }
    for report in reports {
        let namespaces: Vec<_> = report
            .edges
            .iter()
            .filter(|edge| {
                edge.to_file.is_some()
                    && names.get(&edge.id).and_then(|name| name.as_deref()) == Some("*")
            })
            .cloned()
            .collect();
        for namespace in namespaces {
            let mut first = true;
            for ((file, _), id) in &exports {
                if Some(*file) != namespace.to_file {
                    continue;
                }
                let mut edge = namespace.clone();
                edge.to_symbol = Some(*id);
                if first {
                    *report
                        .edges
                        .iter_mut()
                        .find(|edge| edge.id == namespace.id)
                        .expect("namespace edge") = edge;
                    first = false;
                } else {
                    let next = report
                        .edges
                        .iter()
                        .filter(|edge| edge.from_file == namespace.from_file)
                        .map(|edge| edge.id & 0xffff_ffff)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    edge.id = crate::row_id(namespace.from_file, next as usize)?;
                    report.edges.push(edge);
                }
            }
        }
        for edge in &mut report.edges {
            if let Some(target) = edge.to_file
                && let Some(Some(name)) = names.get(&edge.id)
                && name != "*"
            {
                edge.to_symbol = exports.get(&(target, name.clone())).copied();
            }
        }
        let imported_exports: Vec<_> = report
            .edges
            .iter()
            .filter(|edge| edge.kind == "import" && edge.to_symbol.is_some())
            .filter_map(|edge| {
                report
                    .symbols
                    .iter()
                    .find(|symbol| Some(symbol.id) == edge.from_symbol)
                    .map(|symbol| (edge.clone(), symbol.clone()))
            })
            .flat_map(|(edge, binding)| {
                report
                    .symbols
                    .iter()
                    .filter(move |symbol| {
                        symbol.file_id == binding.file_id
                            && symbol.kind == "import"
                            && symbol.exported
                            && symbol.span_start == binding.span_start
                    })
                    .map(move |symbol| (edge.clone(), symbol.id))
            })
            .collect();
        for (mut edge, symbol) in imported_exports {
            edge.kind = "reexport".into();
            edge.from_symbol = Some(symbol);
            let next = report
                .edges
                .iter()
                .filter(|row| row.from_file == edge.from_file)
                .map(|row| row.id & 0xffff_ffff)
                .max()
                .unwrap_or(0)
                + 1;
            edge.id = crate::row_id(edge.from_file, next as usize)?;
            report.edges.push(edge);
        }
    }
    Ok(())
}
