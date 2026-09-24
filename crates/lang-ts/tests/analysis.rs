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
        for claim in expected["supports"].as_array().expect("support list") {
            exercised.insert(claim.as_str().expect("support name").to_owned());
        }

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
        let edges: Vec<_> = report
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
                json!([
                    file,
                    edge.kind,
                    edge.type_only,
                    edge.unresolved_reason,
                    text
                ])
            })
            .collect();
        let unsupported: Vec<_> = report
            .unsupported
            .iter()
            .map(|row| {
                let (file, text) = location(row.file_id, row.span_start, row.span_end);
                json!([file, row.construct, row.reason, text])
            })
            .collect();
        assert_eq!(json!(edges), expected["edges"], "{}", case.display());
        assert_eq!(
            json!(unsupported),
            expected["unsupported"],
            "{}",
            case.display()
        );
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
