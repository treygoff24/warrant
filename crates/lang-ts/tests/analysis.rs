use std::collections::{BTreeMap, BTreeSet};

use warrant_core::nouns::{
    Capture, Entrypoint, InventoryClass, InventoryDocument, InventoryEntry, InventorySummary,
    SnapshotExclusions, SnapshotKind, SnapshotManifest, UnitAliasTable,
};

fn entry(path: &str, class: InventoryClass, language: Option<&str>) -> InventoryEntry {
    InventoryEntry {
        path: path.into(),
        blob: Some(format!("sha1:{path}")),
        class,
        language: language.map(str::to_owned),
        unit: Some(".".into()),
        module: None,
        by: "fixture".into(),
        reason: "fixture".into(),
        entrypoints: Vec::new(),
        unread: None,
        generated_by: None,
        vendored_from: None,
    }
}

fn inventory() -> InventoryDocument {
    let mut index = entry("src/index.ts", InventoryClass::Source, Some("typescript"));
    index.entrypoints.push(Entrypoint {
        kind: "package-export".into(),
        basis: "package.json#exports".into(),
        by: "package.json".into(),
    });
    let entries = vec![
        entry("package.json", InventoryClass::Config, None),
        entry("tsconfig.json", InventoryClass::Config, None),
        index,
        entry("src/leaf.ts", InventoryClass::Source, Some("typescript")),
        entry("src/middle.ts", InventoryClass::Source, Some("typescript")),
        entry(
            "src/index.test.ts",
            InventoryClass::Test,
            Some("typescript"),
        ),
    ];
    InventoryDocument {
        schema_version: "warrant.inventory/1".into(),
        snapshot: SnapshotManifest {
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
        },
        summary: InventorySummary {
            files: entries.len() as u64,
            unit_aliases: vec![UnitAliasTable {
                unit: ".".into(),
                alias_table: Some("tsconfig.json".into()),
                by: "tsconfig".into(),
            }],
            ..InventorySummary::default()
        },
        entries,
        truncated: false,
        total: 6,
        next_cursor: None,
    }
}

#[test]
fn analysis_reports_every_claimed_capability_and_keeps_edges_unresolved() {
    let sources = BTreeMap::from([
        ("package.json", r#"{"exports":"./src/index.ts"}"#),
        ("tsconfig.json", r#"{"compilerOptions":{}}"#),
        (
            "src/index.ts",
            r#"
import type { Shape } from "./leaf";
import { value } from "./leaf";
export { value as publicValue };
export { value as chained } from "./middle";
export type { Shape } from "./leaf";
const common = require("./common");
const lazy = import("./lazy");
const unknown = import(common);
export function use(input: Shape) { return value + input.size + lazy + unknown; }
export default function() { return use({ size: 1 }); }
"#,
        ),
        (
            "src/leaf.ts",
            "export interface Shape { size: number }\nexport const value = 1;",
        ),
        ("src/middle.ts", "export { value } from './leaf';"),
        (
            "src/index.test.ts",
            "import { use } from './index';\nuse({ size: 1 });",
        ),
    ]);
    let read = |path: &str| {
        sources
            .get(path)
            .map(|source| source.as_bytes().to_vec())
            .ok_or_else(|| warrant_lang_ts::ReadError {
                code: "missing-fixture".into(),
                reason: path.into(),
            })
    };
    let inventory = inventory();
    let units = warrant_lang_ts::discover(&inventory);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].config_path.as_deref(), Some("tsconfig.json"));

    let report = warrant_lang_ts::analyze(&units[0], &inventory, &read).expect("analyze fixture");

    assert!(
        report
            .symbols
            .iter()
            .any(|symbol| symbol.name == "use" && symbol.exported)
    );
    assert!(report.symbols.iter().any(|symbol| {
        symbol.name == "value" && symbol.export_name.as_deref() == Some("publicValue")
    }));
    assert!(report.symbols.iter().any(|symbol| {
        symbol.name == "default" && symbol.export_name.as_deref() == Some("default")
    }));
    assert!(report.references.iter().any(|reference| {
        report
            .symbols
            .iter()
            .any(|symbol| symbol.id == reference.symbol_id && symbol.name == "value")
    }));
    for (kind, type_only) in [
        ("import", true),
        ("import", false),
        ("reexport", false),
        ("reexport", true),
        ("require", false),
        ("dynamic_import", false),
    ] {
        assert!(
            report.edges.iter().any(|edge| {
                edge.kind == kind
                    && edge.type_only == type_only
                    && !edge.resolved
                    && edge.to_file.is_none()
                    && edge.to_symbol.is_none()
                    && edge.to_external.is_none()
                    && edge.unresolved_reason.as_deref() == Some("resolution-pending")
            }),
            "missing {kind} edge"
        );
    }
    assert!(report.unsupported.iter().any(|unsupported| {
        unsupported.construct == "dynamic-import-nonliteral"
            && unsupported.reason == "dynamic-nonliteral"
    }));
    assert!(
        report
            .entrypoints
            .iter()
            .any(|entrypoint| entrypoint.kind == "package-export")
    );
    assert!(
        report
            .entrypoints
            .iter()
            .any(|entrypoint| entrypoint.kind == "test-file")
    );

    let capabilities = warrant_lang_ts::capabilities();
    assert_eq!(capabilities.symbol_level, "binding");
    assert_eq!(capabilities.resolution_authority, "unqualified");
    let expected = BTreeSet::from([
        "cjs-require-literal".to_owned(),
        "dynamic-import-literal".to_owned(),
        "esm-import".to_owned(),
        "esm-reexport".to_owned(),
    ]);
    assert_eq!(
        capabilities.supports.into_iter().collect::<BTreeSet<_>>(),
        expected
    );
    assert!(capabilities.unsupported.iter().any(|claim| {
        claim.construct == "dynamic-import-nonliteral" && claim.treatment == "dynamic-nonliteral"
    }));
}

#[test]
fn conformance_fixtures_cover_every_support_claim() {
    use serde_json::{Value, json};
    use std::{fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/model");
    let mut exercised = BTreeSet::new();
    for case in fs::read_dir(root).expect("model fixtures") {
        let case = case.expect("fixture").path();
        let expected: Value = serde_json::from_slice(
            &fs::read(case.join("analysis.json")).expect("analysis expectation"),
        )
        .expect("valid analysis expectation");

        let mut paths: Vec<_> = fs::read_dir(case.join("repo"))
            .expect("fixture repo")
            .map(|file| file.expect("fixture file").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
            .collect();
        paths.sort();
        let mut fixture = inventory();
        fixture.entries = paths
            .iter()
            .map(|path| {
                entry(
                    path.file_name()
                        .expect("file name")
                        .to_str()
                        .expect("UTF-8 file name"),
                    InventoryClass::Source,
                    Some("typescript"),
                )
            })
            .collect();
        fixture.summary.files = fixture.entries.len() as u64;
        fixture.total = fixture.entries.len() as u64;
        let sources: BTreeMap<_, _> = paths
            .iter()
            .map(|path| {
                (
                    path.file_name()
                        .expect("file name")
                        .to_str()
                        .expect("UTF-8 file name")
                        .to_owned(),
                    fs::read(path).expect("fixture source"),
                )
            })
            .collect();
        let read = |path: &str| {
            sources
                .get(path)
                .cloned()
                .ok_or_else(|| warrant_lang_ts::ReadError {
                    code: "missing-fixture".into(),
                    reason: path.into(),
                })
        };
        let unit = warrant_lang_ts::discover(&fixture)
            .into_iter()
            .next()
            .expect("TypeScript unit");
        let report = warrant_lang_ts::analyze(&unit, &fixture, &read).expect("analyze fixture");
        for edge in &report.edges {
            let claim = match (edge.kind.as_str(), edge.unresolved_reason.as_deref()) {
                ("import", _) => Some("esm-import"),
                ("reexport", _) => Some("esm-reexport"),
                ("require", Some("resolution-pending")) => Some("cjs-require-literal"),
                ("dynamic_import", Some("resolution-pending")) => Some("dynamic-import-literal"),
                _ => None,
            };
            if let Some(claim) = claim {
                exercised.insert(claim.to_owned());
            }
        }
        let location = |file_id, start: i64, end: i64| {
            let file = report
                .files
                .iter()
                .find(|file| file.id == file_id)
                .expect("file row");
            let source = sources.get(&file.path).expect("source bytes");
            (
                file.path.as_str(),
                std::str::from_utf8(&source[start as usize..end as usize]).expect("UTF-8 span"),
            )
        };
        let mut edges: Vec<_> = report
            .edges
            .iter()
            .map(|edge| {
                assert!(
                    !edge.resolved
                        && edge.to_file.is_none()
                        && edge.to_symbol.is_none()
                        && edge.to_external.is_none()
                );
                let (file, text) = location(edge.from_file, edge.span_start, edge.span_end);
                assert!(!text.is_empty());
                assert!(
                    expected["edges"]
                        .as_array()
                        .expect("edges")
                        .iter()
                        .any(|row| {
                            row[0] == file
                                && row[1] == edge.kind
                                && row[2] == edge.type_only
                                && row[3] == json!(edge.unresolved_reason)
                                && span_within(
                                    &sources[file],
                                    row[4].as_str().expect("syntax"),
                                    edge.span_start,
                                    edge.span_end,
                                )
                        }),
                    "edge span must stay within its fixture syntax"
                );
                json!([file, edge.kind, edge.type_only, edge.unresolved_reason])
            })
            .collect();
        let mut unsupported: Vec<_> = report
            .unsupported
            .iter()
            .map(|row| {
                let (file, text) = location(row.file_id, row.span_start, row.span_end);
                assert!(!text.is_empty());
                assert!(
                    expected["unsupported"]
                        .as_array()
                        .expect("unsupported")
                        .iter()
                        .any(|expected| {
                            expected[0] == file
                                && expected[1] == row.construct
                                && expected[2] == row.reason
                                && span_within(
                                    &sources[file],
                                    expected[3].as_str().expect("syntax"),
                                    row.span_start,
                                    row.span_end,
                                )
                        }),
                    "unsupported span must stay within its fixture syntax"
                );
                json!([file, row.construct, row.reason])
            })
            .collect();
        let mut expected_edges: Vec<_> = expected["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .map(|row| Value::Array(row.as_array().expect("edge row")[..4].to_vec()))
            .collect();
        let mut expected_unsupported: Vec<_> = expected["unsupported"]
            .as_array()
            .expect("unsupported")
            .iter()
            .map(|row| Value::Array(row.as_array().expect("unsupported row")[..3].to_vec()))
            .collect();
        edges.sort_by_key(Value::to_string);
        expected_edges.sort_by_key(Value::to_string);
        unsupported.sort_by_key(Value::to_string);
        expected_unsupported.sort_by_key(Value::to_string);
        assert_eq!(edges, expected_edges, "{}", case.display());
        assert_eq!(unsupported, expected_unsupported, "{}", case.display());
    }
    assert_eq!(
        exercised,
        warrant_lang_ts::capabilities()
            .supports
            .into_iter()
            .collect()
    );
}

fn analyze_source(source: &str) -> warrant_model::IntegrationReport {
    let mut fixture = inventory();
    fixture.entries = vec![entry(
        "index.ts",
        InventoryClass::Source,
        Some("typescript"),
    )];
    fixture.summary.files = 1;
    fixture.total = 1;
    let read = |_: &str| Ok(source.as_bytes().to_vec());
    let unit = warrant_lang_ts::discover(&fixture)
        .into_iter()
        .next()
        .expect("TypeScript unit");
    warrant_lang_ts::analyze(&unit, &fixture, &read).expect("analyze source")
}

#[test]
fn unsupported_forms_are_explicit_and_shadowed_require_is_not_a_load() {
    let report = analyze_source(
        "namespace Hidden { export const x = 1; }\n@decorate class Example {}\nmodule.exports = Example;\n",
    );
    for construct in ["ts-namespace", "decorator", "cjs-exports"] {
        assert!(
            report
                .unsupported
                .iter()
                .any(|row| row.construct == construct),
            "missing {construct}"
        );
    }
    assert!(
        !report
            .symbols
            .iter()
            .any(|symbol| { symbol.name == "x" && symbol.exported })
    );

    let shadowed = analyze_source(
        "function run(require: (path: string) => void) { require('./not-a-load'); }",
    );
    assert!(shadowed.edges.is_empty());
    assert!(
        analyze_source("export { missing };")
            .unsupported
            .iter()
            .any(|row| row.construct == "unbound-export")
    );
}

#[test]
fn symbol_ids_are_unique_across_files_with_export_aliases() {
    let mut fixture = inventory();
    fixture.entries = vec![
        entry("prior.ts", InventoryClass::Source, Some("typescript")),
        entry("index.ts", InventoryClass::Source, Some("typescript")),
    ];
    let read = |path: &str| {
        Ok(if path == "prior.ts" {
            b"const prior = 1;".to_vec()
        } else {
            b"export const value = 1; export { value as alias }; export default function() {}"
                .to_vec()
        })
    };
    let unit = warrant_lang_ts::discover(&fixture).remove(0);
    let report = warrant_lang_ts::analyze(&unit, &fixture, &read).expect("analyze files");
    let mut ids = BTreeSet::new();
    for symbol in report.symbols {
        assert!(ids.insert(symbol.id), "duplicate id {}", symbol.id);
    }
}

#[test]
fn local_function_calls_have_binding_references() {
    let source = "function local() {} local();";
    let report = analyze_source(source);
    let symbol = report
        .symbols
        .iter()
        .find(|symbol| symbol.name == "local")
        .expect("local binding");
    let references: Vec<_> = report
        .references
        .iter()
        .filter(|row| row.symbol_id == symbol.id)
        .collect();
    assert_eq!(references.len(), 1, "local call reference count");
    let reference = references[0];
    assert_eq!(
        &source[reference.span_start as usize..reference.span_end as usize],
        "local"
    );
    assert_eq!(reference.file_id, symbol.file_id);
}

#[test]
fn invalid_encoding_preserves_sibling_analysis() {
    let mut fixture = inventory();
    fixture.entries = vec![
        entry("invalid.ts", InventoryClass::Source, Some("typescript")),
        entry("sibling.ts", InventoryClass::Source, Some("typescript")),
    ];
    let read = |path: &str| {
        Ok(if path == "invalid.ts" {
            vec![0xff]
        } else {
            b"export const sibling = 1;".to_vec()
        })
    };
    let unit = warrant_lang_ts::discover(&fixture).remove(0);
    let report = warrant_lang_ts::analyze(&unit, &fixture, &read)
        .expect("invalid source does not abort unit");
    assert!(
        report
            .symbols
            .iter()
            .any(|row| row.name == "sibling" && row.file_id == 2)
    );
    assert!(report.unsupported.iter().any(|row| row.file_id == 1
        && row.construct == "source-encoding"
        && row.reason == "invalid-encoding"));
    assert!(
        warrant_lang_ts::capabilities()
            .unsupported
            .iter()
            .any(|row| row.construct == "source-encoding" && row.treatment == "invalid-encoding")
    );
}

#[test]
fn discovery_partitions_root_and_nested_units() {
    let mut fixture = inventory();
    let mut nested = entry(
        "packages/x/index.ts",
        InventoryClass::Source,
        Some("typescript"),
    );
    nested.unit = Some("packages/x".into());
    fixture.entries.push(nested);
    fixture.entries.push(entry(
        "packages/x/tsconfig.json",
        InventoryClass::Config,
        None,
    ));
    fixture.summary.unit_aliases.push(UnitAliasTable {
        unit: "packages/x".into(),
        alias_table: Some("packages/x/tsconfig.json".into()),
        by: "tsconfig".into(),
    });
    let units = warrant_lang_ts::discover(&fixture);
    assert_eq!(
        units
            .iter()
            .map(|unit| (
                unit.root.as_str(),
                unit.config_path.as_deref(),
                unit.kind.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (".", Some("tsconfig.json"), "tsconfig"),
            ("packages/x", Some("packages/x/tsconfig.json"), "tsconfig")
        ]
    );
    let read = |_: &str| Ok(b"export const value = 1;".to_vec());
    for unit in &units {
        let report = warrant_lang_ts::analyze(unit, &fixture, &read).expect("unit analysis");
        let expected: BTreeSet<_> = fixture
            .entries
            .iter()
            .filter(|entry| {
                entry.language.as_deref() == Some("typescript")
                    && entry.unit.as_deref() == Some(&unit.root)
            })
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(
            report
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(report.files.iter().all(|file| file.unit_id == unit.id));
    }
    fixture.summary.unit_aliases.clear();
    fixture.entries.retain(|entry| entry.language.is_some());
    let units = warrant_lang_ts::discover(&fixture);
    assert_eq!(units.len(), 2);
    assert!(
        units
            .iter()
            .all(|unit| unit.kind == "source-root" && unit.config_path.is_none())
    );
}

#[test]
fn unsupported_treatments_have_observed_fixtures() {
    for (source, construct, reason) in [
        ("const = ;", "parse-error", "invalid-syntax"),
        (
            "let value; let value;",
            "semantic-error",
            "invalid-bindings",
        ),
        ("require(name);", "require-nonliteral", "dynamic-nonliteral"),
        (
            "type Shape = import('./leaf').Shape;",
            "ts-import-type",
            "not-observed",
        ),
        (
            "export as namespace Library;",
            "ts-namespace-export",
            "not-observed",
        ),
    ] {
        let report = analyze_source(source);
        assert!(
            report
                .unsupported
                .iter()
                .any(|row| row.construct == construct && row.reason == reason),
            "missing {construct}: {:?}",
            report.unsupported
        );
        assert!(
            warrant_lang_ts::capabilities()
                .unsupported
                .iter()
                .any(|row| row.construct == construct && row.treatment == reason)
        );
    }
    let mut fixture = inventory();
    fixture.entries = vec![entry(
        "index.unknown",
        InventoryClass::Source,
        Some("typescript"),
    )];
    let unit = warrant_lang_ts::discover(&fixture).remove(0);
    let report = warrant_lang_ts::analyze(&unit, &fixture, &|_| Ok(b"const value = 1;".to_vec()))
        .expect("unsupported source type");
    assert!(
        report
            .unsupported
            .iter()
            .any(|row| row.construct == "source-type" && row.reason == "unsupported-extension")
    );
}

fn span_within(source: &[u8], syntax: &str, start: i64, end: i64) -> bool {
    std::str::from_utf8(source)
        .expect("fixture source")
        .match_indices(syntax)
        .any(|(offset, text)| offset <= start as usize && end as usize <= offset + text.len())
}
