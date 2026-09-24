//! TypeScript and JavaScript syntax and binding analysis over captured source bytes.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error as StdError,
    fmt,
    path::Path,
};

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Argument, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};
use warrant_core::nouns::{
    CapabilityReport, InventoryClass, InventoryDocument, UnsupportedCapability,
};
use warrant_model::{
    CapabilityReportRow, EdgeRow, EntrypointRow, FileRow, IntegrationReport, ModuleRow,
    ReferenceRow, SymbolRow, UnitRow, UnsupportedRow,
};

pub mod frameworks;
pub mod parity;
pub mod references;
mod resolution;
pub use resolution::resolve;

const INTEGRATION: &str = "lang-ts";

/// A captured-source read refusal, kept in the snapshot's vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadError {
    pub code: String,
    pub reason: String,
}

/// One TypeScript or JavaScript compilation/package unit selected by inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unit {
    pub id: i64,
    pub root: String,
    pub config_path: Option<String>,
    pub kind: String,
}

#[derive(Debug)]
pub enum AnalysisError {
    Read { path: String, source: ReadError },
    Invalid(String),
    Json(serde_json::Error),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "could not read `{path}`: {}: {}",
                    source.code, source.reason
                )
            }
            Self::Invalid(reason) => formatter.write_str(reason),
            Self::Json(error) => write!(formatter, "could not encode capability report: {error}"),
        }
    }
}

impl StdError for AnalysisError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Read { .. } | Self::Invalid(_) => None,
        }
    }
}

impl From<serde_json::Error> for AnalysisError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type Result<T> = std::result::Result<T, AnalysisError>;
pub type Reader<'a> = &'a dyn Fn(&str) -> std::result::Result<Vec<u8>, ReadError>;

/// Select exactly the inventory units that contain captured TypeScript/JavaScript files.
pub fn discover(inventory: &InventoryDocument) -> Vec<Unit> {
    let mut roots = inventory
        .entries
        .iter()
        .filter(|entry| is_analyzable(entry))
        .map(|entry| entry.unit.clone().unwrap_or_else(|| ".".into()))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .enumerate()
        .map(|(index, root)| {
            let alias = inventory
                .summary
                .unit_aliases
                .iter()
                .find(|row| row.unit == root)
                .and_then(|row| row.alias_table.clone());
            let package_path = within(&root, "package.json");
            let package = inventory
                .entries
                .iter()
                .map(|entry| entry.path.as_str())
                .find(|path| *path == package_path);
            let tsconfig = within(&root, "tsconfig.json");
            let config_path = alias
                .filter(|path| is_tsconfig(path))
                .or_else(|| {
                    inventory
                        .entries
                        .iter()
                        .any(|entry| entry.path == tsconfig)
                        .then_some(tsconfig)
                })
                .or_else(|| package.map(str::to_owned));
            let kind = match config_path.as_deref() {
                Some(path) if is_tsconfig(path) => "tsconfig",
                Some(_) => "package",
                None => "source-root",
            };
            Unit {
                id: i64::try_from(index + 1).expect("unit count fits SQLite integer"),
                root,
                config_path,
                kind: kind.into(),
            }
        })
        .collect()
}

/// Deterministic module rows matching the ids used by [`analyze`].
pub fn module_rows(inventory: &InventoryDocument) -> Vec<ModuleRow> {
    let mut names = inventory
        .entries
        .iter()
        .filter_map(|entry| entry.module.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| ModuleRow {
            id: i64::try_from(index + 1).expect("module count fits SQLite integer"),
            name,
            declared_by: "inventory".into(),
            intent: None,
            owner: None,
        })
        .collect()
}

/// Analyze syntax in one inventory unit; pass all unit reports to [`resolve`].
pub fn analyze(
    unit: &Unit,
    inventory: &InventoryDocument,
    read: Reader<'_>,
) -> Result<IntegrationReport> {
    let modules = module_rows(inventory)
        .into_iter()
        .map(|row| (row.name, row.id))
        .collect::<BTreeMap<_, _>>();
    let mut report = IntegrationReport {
        unit: UnitRow {
            id: unit.id,
            integration: INTEGRATION.into(),
            root: unit.root.clone(),
            config_path: unit.config_path.clone(),
            kind: unit.kind.clone(),
        },
        files: Vec::new(),
        symbols: Vec::new(),
        edges: Vec::new(),
        references: Vec::new(),
        entrypoints: Vec::new(),
        effects: Vec::new(),
        unsupported: Vec::new(),
        capability_report: CapabilityReportRow {
            integration: INTEGRATION.into(),
            json: serde_json::to_value(capabilities())?,
        },
    };

    for (entry_index, entry) in inventory.entries.iter().enumerate().filter(|(_, entry)| {
        is_analyzable(entry) && entry.unit.as_deref().unwrap_or(".") == unit.root
    }) {
        let file_id = i64::try_from(entry_index + 1)
            .map_err(|_| AnalysisError::Invalid("inventory has too many files".into()))?;
        let blob = entry.blob.clone().ok_or_else(|| {
            AnalysisError::Invalid(format!("captured source `{}` has no blob", entry.path))
        })?;
        report.files.push(FileRow {
            id: file_id,
            path: entry.path.clone(),
            blob,
            class: entry.class.as_str().into(),
            language: entry.language.clone(),
            unit_id: unit.id,
            module_id: entry
                .module
                .as_ref()
                .and_then(|name| modules.get(name))
                .copied(),
        });
        for declared in &entry.entrypoints {
            report.entrypoints.push(EntrypointRow {
                id: row_id(file_id, report.entrypoints.len() + 1)?,
                file_id,
                symbol_id: None,
                kind: declared.kind.clone(),
                basis: declared.basis.clone(),
                declared_by: Some(declared.by.clone()),
            });
        }
        if entry.class == InventoryClass::Test {
            report.entrypoints.push(EntrypointRow {
                id: row_id(file_id, report.entrypoints.len() + 1)?,
                file_id,
                symbol_id: None,
                kind: "test-file".into(),
                basis: "test-file-convention".into(),
                declared_by: None,
            });
        }
        let bytes = read(&entry.path).map_err(|source| AnalysisError::Read {
            path: entry.path.clone(),
            source,
        })?;
        let Ok(source) = std::str::from_utf8(&bytes) else {
            push_unsupported(
                &mut report,
                file_id,
                Span::new(0, bytes.len() as u32),
                "source-encoding",
                "invalid-encoding",
            )?;
            continue;
        };
        analyze_file(file_id, &entry.path, source, &mut report)?;
    }

    parity::augment(unit, inventory, read, &mut report)?;
    frameworks::augment(unit, inventory, read, &mut report)?;
    references::augment(unit, inventory, read, &mut report)?;
    Ok(report)
}

/// Capability claims for the syntax-and-binding fast path.
pub fn capabilities() -> CapabilityReport {
    CapabilityReport {
        schema_version: "warrant.capabilities/1".into(),
        integration: INTEGRATION.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        instruments: BTreeMap::from([
            ("oxc_parser".into(), "0.150.0".into()),
            ("oxc_semantic".into(), "0.150.0".into()),
            ("oxc_resolver".into(), "11.24.3".into()),
        ]),
        resolution_oracle: None,
        compiler_reference_instrument: None,
        resolution_authority: "unqualified".into(),
        resolution_modes_qualified: Vec::new(),
        symbol_level: "binding".into(),
        type_only_distinction: true,
        supports: [
            "cjs-require-literal",
            "dynamic-import-literal",
            "esm-import",
            "esm-reexport",
            "tsconfig-paths",
            "package-exports",
            "package-imports",
            "project-references",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        unsupported: [
            ("star-reexport", "ambiguous-star-export"),
            ("dynamic-import-nonliteral", "dynamic-nonliteral"),
            ("require-nonliteral", "dynamic-nonliteral"),
            ("ts-import-equals", "not-observed"),
            ("ts-export-assignment", "not-observed"),
            ("ts-namespace-export", "not-observed"),
            ("ts-import-type", "not-observed"),
            ("ts-namespace", "not-observed"),
            ("decorator", "requires-declaration"),
            ("cjs-exports", "not-observed"),
            ("parse-error", "invalid-syntax"),
            ("semantic-error", "invalid-bindings"),
            ("unbound-export", "binding-not-found"),
            ("source-type", "unsupported-extension"),
            ("source-encoding", "invalid-encoding"),
        ]
        .into_iter()
        .map(|(construct, treatment)| UnsupportedCapability {
            construct: construct.into(),
            treatment: treatment.into(),
        })
        .collect(),
        limits: "Module resolution is unqualified pending S6. Binding consumers exclude type-derived references, reflection and execution order. Only captured dependency files participate in resolution; symlink identity is not established.".into(),
    }
}

fn analyze_file(
    file_id: i64,
    path: &str,
    source: &str,
    report: &mut IntegrationReport,
) -> Result<()> {
    let allocator = Allocator::default();
    let Ok(source_type) = SourceType::from_path(Path::new(path)) else {
        push_unsupported(
            report,
            file_id,
            Span::new(0, source.len() as u32),
            "source-type",
            "unsupported-extension",
        )?;
        return Ok(());
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        push_unsupported(
            report,
            file_id,
            Span::new(0, source.len() as u32),
            "parse-error",
            "invalid-syntax",
        )?;
        return Ok(());
    }
    let semantic_return = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(allocator.alloc(parsed.program));
    if !semantic_return.diagnostics.is_empty() {
        push_unsupported(
            report,
            file_id,
            Span::new(0, source.len() as u32),
            "semantic-error",
            "invalid-bindings",
        )?;
        return Ok(());
    }
    let semantic = semantic_return.semantic;
    let mut exports = Vec::<(String, String, Span)>::new();
    let mut export_spans = Vec::<(Span, Option<String>)>::new();

    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ExportNamedDeclaration(declaration) => {
                for specifier in &declaration.specifiers {
                    exports.push((
                        specifier.local.name().to_string(),
                        specifier.exported.name().to_string(),
                        specifier.span,
                    ));
                }
            }
            AstKind::ExportDeclaration(declaration) => {
                export_spans.push((declaration.span, None));
            }
            AstKind::ExportDefaultDeclaration(declaration) => {
                if let ExportDefaultDeclarationKind::Identifier(identifier) =
                    &declaration.declaration
                {
                    exports.push((
                        identifier.name.to_string(),
                        "default".into(),
                        declaration.span,
                    ));
                } else {
                    export_spans.push((declaration.span, Some("default".into())));
                }
            }
            _ => {}
        }
    }

    let symbol_offset = report.symbols.len();
    let mut binding_ids = BTreeMap::<String, i64>::new();
    let mut root_binding_ids = BTreeMap::<String, i64>::new();
    let mut matched_default_spans = BTreeSet::new();
    let root_scope = semantic.scoping().root_scope_id();
    for (local_index, symbol_id) in semantic.scoping().symbol_ids().enumerate() {
        let name = semantic.scoping().symbol_name(symbol_id).to_owned();
        let span = semantic.scoping().symbol_span(symbol_id);
        let flags = semantic.scoping().symbol_flags(symbol_id);
        let is_root = semantic.symbol_scope(symbol_id) == root_scope;
        let span_export = is_root
            .then(|| {
                export_spans
                    .iter()
                    .find(|(export_span, _)| contains(*export_span, span))
            })
            .flatten();
        let export_name = span_export
            .and_then(|(_, name)| name.clone())
            .or_else(|| span_export.map(|_| name.clone()));
        if export_name.as_deref() == Some("default")
            && let Some((export_span, _)) = span_export
        {
            matched_default_spans.insert(export_span.start);
        }
        let id = row_id(file_id, local_index + 1)?;
        if flags.is_import() {
            binding_ids.entry(name.clone()).or_insert(id);
        }
        if is_root {
            root_binding_ids.insert(name.clone(), id);
        }
        let kind = if flags.is_import() {
            "import"
        } else if flags.is_function() {
            "function"
        } else if flags.is_class() {
            "class"
        } else if flags.is_interface() {
            "interface"
        } else if flags.is_type_alias() {
            "type-alias"
        } else if flags.is_enum() {
            "enum"
        } else if flags.is_namespace_module() || flags.is_value_module() {
            "namespace"
        } else {
            "binding"
        };
        let visibility = if export_name.is_some() {
            "public"
        } else {
            "local"
        };
        report.symbols.push(SymbolRow {
            id,
            file_id,
            name,
            kind: kind.into(),
            exported: export_name.is_some(),
            export_name,
            visibility: visibility.into(),
            span_start: i64::from(span.start),
            span_end: i64::from(span.end),
        });
        for reference in semantic.symbol_references(symbol_id) {
            let span = semantic.reference_span(reference);
            report.references.push(ReferenceRow {
                id: row_id(file_id, report.references.len() + 1)?,
                file_id,
                symbol_id: id,
                span_start: i64::from(span.start),
                span_end: i64::from(span.end),
            });
        }
    }

    for (local, exported, span) in exports {
        if let Some(symbol) = root_binding_ids.get(&local).copied() {
            export_binding(report, file_id, symbol_offset, symbol, &exported)?;
        } else {
            push_unsupported(report, file_id, span, "unbound-export", "binding-not-found")?;
        }
    }

    for (span, name) in &export_spans {
        if name.as_deref() != Some("default") || matched_default_spans.contains(&span.start) {
            continue;
        }
        let id = row_id(file_id, report.symbols.len() - symbol_offset + 1)?;
        report.symbols.push(SymbolRow {
            id,
            file_id,
            name: "default".into(),
            kind: "export".into(),
            exported: true,
            export_name: Some("default".into()),
            visibility: "public".into(),
            span_start: i64::from(span.start),
            span_end: i64::from(span.end),
        });
    }

    let mut next_symbol = report.symbols.len() - symbol_offset + 1;
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ImportDeclaration(declaration) => {
                let declaration_type_only = declaration.import_kind.is_type();
                match declaration.specifiers.as_deref() {
                    Some(specifiers) if !specifiers.is_empty() => {
                        for imported in specifiers {
                            let name = imported.name().to_string();
                            let type_only = declaration_type_only
                                || matches!(imported, ImportDeclarationSpecifier::ImportSpecifier(specifier) if specifier.import_kind.is_type());
                            push_edge(
                                report,
                                file_id,
                                "import",
                                binding_ids.get(&name).copied(),
                                None,
                                type_only,
                                imported.span(),
                                "resolution-pending",
                            )?;
                        }
                    }
                    _ => push_edge(
                        report,
                        file_id,
                        "import",
                        None,
                        None,
                        false,
                        declaration.span,
                        "resolution-pending",
                    )?,
                }
            }
            AstKind::ExportFromDeclaration(declaration) => {
                for specifier in &declaration.specifiers {
                    let exported = specifier.exported.name().to_string();
                    let synthetic = row_id(file_id, next_symbol)?;
                    next_symbol += 1;
                    report.symbols.push(SymbolRow {
                        id: synthetic,
                        file_id,
                        name: exported.clone(),
                        kind: "reexport".into(),
                        exported: true,
                        export_name: Some(exported),
                        visibility: "public".into(),
                        span_start: i64::from(specifier.span.start),
                        span_end: i64::from(specifier.span.end),
                    });
                    push_edge(
                        report,
                        file_id,
                        "reexport",
                        Some(synthetic),
                        None,
                        declaration.export_kind.is_type() || specifier.export_kind.is_type(),
                        specifier.span,
                        "resolution-pending",
                    )?;
                }
            }
            AstKind::ExportAllDeclaration(declaration) => {
                let symbol = if let Some(exported) = &declaration.exported {
                    let id = row_id(file_id, next_symbol)?;
                    next_symbol += 1;
                    report.symbols.push(SymbolRow {
                        id,
                        file_id,
                        name: "*".into(),
                        kind: "reexport".into(),
                        exported: true,
                        export_name: Some(exported.name().to_string()),
                        visibility: "public".into(),
                        span_start: i64::from(declaration.span.start),
                        span_end: i64::from(declaration.span.end),
                    });
                    Some(id)
                } else {
                    None
                };
                push_edge(
                    report,
                    file_id,
                    "reexport",
                    symbol,
                    None,
                    declaration.export_kind.is_type(),
                    declaration.span,
                    "resolution-pending",
                )?;
            }
            AstKind::CallExpression(call) => {
                if let Expression::Identifier(identifier) = &call.callee
                    && identifier.name == "require"
                    && semantic.is_reference_to_global_variable(identifier)
                {
                    let literal = call.arguments.len() == 1
                        && matches!(call.arguments.first(), Some(Argument::StringLiteral(_)));
                    push_edge(
                        report,
                        file_id,
                        "require",
                        None,
                        None,
                        false,
                        call.span,
                        if literal {
                            "resolution-pending"
                        } else {
                            "dynamic-nonliteral"
                        },
                    )?;
                    if !literal {
                        push_unsupported(
                            report,
                            file_id,
                            call.span,
                            "require-nonliteral",
                            "dynamic-nonliteral",
                        )?;
                    }
                }
            }
            AstKind::ImportExpression(import) => match &import.source {
                Expression::StringLiteral(_) => push_edge(
                    report,
                    file_id,
                    "dynamic_import",
                    None,
                    None,
                    false,
                    import.span,
                    "resolution-pending",
                )?,
                _ => {
                    push_edge(
                        report,
                        file_id,
                        "dynamic_import",
                        None,
                        None,
                        false,
                        import.span,
                        "dynamic-nonliteral",
                    )?;
                    push_unsupported(
                        report,
                        file_id,
                        import.span,
                        "dynamic-import-nonliteral",
                        "dynamic-nonliteral",
                    )?;
                }
            },
            AstKind::TSImportEqualsDeclaration(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-import-equals",
                "not-observed",
            )?,
            AstKind::TSExportAssignment(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-export-assignment",
                "not-observed",
            )?,
            AstKind::TSNamespaceExportDeclaration(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-namespace-export",
                "not-observed",
            )?,
            AstKind::TSNamespaceDeclaration(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-namespace",
                "not-observed",
            )?,
            AstKind::TSExternalModuleDeclaration(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-namespace",
                "not-observed",
            )?,
            AstKind::TSImportType(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "ts-import-type",
                "not-observed",
            )?,
            AstKind::Decorator(declaration) => push_unsupported(
                report,
                file_id,
                declaration.span,
                "decorator",
                "requires-declaration",
            )?,
            AstKind::StaticMemberExpression(member) => {
                if let Expression::Identifier(identifier) = &member.object
                    && (identifier.name == "exports"
                        || (identifier.name == "module" && member.property.name == "exports"))
                    && semantic.is_reference_to_global_variable(identifier)
                {
                    push_unsupported(report, file_id, member.span, "cjs-exports", "not-observed")?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_edge(
    report: &mut IntegrationReport,
    file_id: i64,
    kind: &str,
    from_symbol: Option<i64>,
    to_external: Option<String>,
    type_only: bool,
    span: Span,
    reason: &str,
) -> Result<()> {
    report.edges.push(EdgeRow {
        id: row_id(file_id, report.edges.len() + 1)?,
        kind: kind.into(),
        from_file: file_id,
        from_symbol,
        to_file: None,
        to_symbol: None,
        to_external,
        resolved: false,
        unresolved_reason: Some(reason.into()),
        type_only,
        span_start: i64::from(span.start),
        span_end: i64::from(span.end),
        basis: "observed-static".into(),
    });
    Ok(())
}

fn export_binding(
    report: &mut IntegrationReport,
    file_id: i64,
    symbol_offset: usize,
    symbol_id: i64,
    export_name: &str,
) -> Result<()> {
    let next_id = row_id(file_id, report.symbols.len() - symbol_offset + 1)?;
    let symbol = report
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == symbol_id)
        .expect("symbol was recorded");
    if symbol.exported && symbol.export_name.as_deref() != Some(export_name) {
        let mut alias = symbol.clone();
        alias.id = next_id;
        alias.export_name = Some(export_name.into());
        alias.visibility = "public".into();
        report.symbols.push(alias);
    } else {
        symbol.exported = true;
        symbol.export_name = Some(export_name.into());
        symbol.visibility = "public".into();
    }
    Ok(())
}

fn push_unsupported(
    report: &mut IntegrationReport,
    file_id: i64,
    span: Span,
    construct: &str,
    reason: &str,
) -> Result<()> {
    report.unsupported.push(UnsupportedRow {
        id: row_id(file_id, report.unsupported.len() + 1)?,
        file_id,
        construct: construct.into(),
        span_start: i64::from(span.start),
        span_end: i64::from(span.end),
        reason: reason.into(),
    });
    Ok(())
}

fn row_id(file_id: i64, local: usize) -> Result<i64> {
    let local = i64::try_from(local)
        .map_err(|_| AnalysisError::Invalid("file has too many analysis rows".into()))?;
    file_id
        .checked_shl(32)
        .and_then(|prefix| prefix.checked_add(local))
        .ok_or_else(|| AnalysisError::Invalid("analysis row id overflow".into()))
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn within(root: &str, name: &str) -> String {
    if root == "." {
        name.into()
    } else {
        format!("{root}/{name}")
    }
}

fn is_tsconfig(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("tsconfig") && name.ends_with(".json"))
}

fn is_analyzable(entry: &warrant_core::nouns::InventoryEntry) -> bool {
    entry.language.as_deref() == Some("typescript")
        && entry.blob.is_some()
        && !matches!(
            entry.class,
            InventoryClass::Ignored
                | InventoryClass::Unread
                | InventoryClass::BuildOutput
                | InventoryClass::Vendored
        )
}
