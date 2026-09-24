use std::collections::BTreeMap;
use warrant_core::nouns::{InventoryClass, InventoryDocument, InventoryEntry};
use warrant_model::IntegrationReport;

fn resolve(files: &[(&str, &str)]) -> (InventoryDocument, Vec<IntegrationReport>) {
    let sources: BTreeMap<_, _> = files.iter().copied().collect();
    let mut inventory: InventoryDocument = serde_json::from_value(serde_json::json!({
        "schema_version": "warrant.inventory/1",
        "snapshot": {
            "schema_version": "warrant.snapshot/1", "repo": "sha1:repo", "kind": "worktree",
            "tree": "sha1:tree", "object_format": "sha1", "commit": null,
            "capture": {"kind": "fixture", "manifest_digest": null}, "excluded": warrant_core::nouns::SnapshotExclusions::default(), "taken_at": "fixture"
        },
        "summary": warrant_core::nouns::InventorySummary::default(), "entries": [], "truncated": false, "total": 0, "next_cursor": null
    })).expect("inventory");
    inventory.entries = files
        .iter()
        .map(|(path, _)| {
            let source = path.ends_with(".ts") || path.ends_with(".mts") || path.ends_with(".cts");
            InventoryEntry {
                path: (*path).into(),
                blob: Some(format!("sha1:{path}")),
                class: if source {
                    InventoryClass::Source
                } else {
                    InventoryClass::Config
                },
                language: source.then(|| "typescript".into()),
                unit: Some(".".into()),
                module: source.then(|| (*path).into()),
                by: "fixture".into(),
                reason: "fixture".into(),
                entrypoints: vec![],
                unread: None,
                generated_by: None,
                vendored_from: None,
            }
        })
        .collect();
    if sources.contains_key("tsconfig.json") {
        inventory
            .summary
            .unit_aliases
            .push(warrant_core::nouns::UnitAliasTable {
                unit: ".".into(),
                alias_table: Some("tsconfig.json".into()),
                by: "fixture".into(),
            });
    }
    let read = |path: &str| {
        sources
            .get(path)
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| warrant_lang_ts::ReadError {
                code: "missing".into(),
                reason: path.into(),
            })
    };
    let mut reports: Vec<_> = warrant_lang_ts::discover(&inventory)
        .iter()
        .map(|unit| warrant_lang_ts::analyze(unit, &inventory, &read).expect("syntax"))
        .collect();
    warrant_lang_ts::resolve(&mut reports, &inventory, &read).expect("resolution");
    (inventory, reports)
}

#[test]
fn paths_resolve_with_exported_bindings_and_missing_edges_keep_reasons() {
    let (_, reports) = resolve(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
        ),
        (
            "index.ts",
            "import { value as local } from '@/leaf'; import './missing';",
        ),
        ("src/leaf.ts", "export const value = 1;"),
    ]);
    let report = &reports[0];
    let edge = &report.edges[0];
    assert!(edge.resolved, "{edge:?}");
    assert_eq!(edge.to_file, Some(3));
    let symbol = report
        .symbols
        .iter()
        .find(|symbol| Some(symbol.id) == edge.to_symbol)
        .expect("target binding");
    assert_eq!(symbol.export_name.as_deref(), Some("value"));
    assert!(!report.edges[1].resolved);
    assert!(
        report.edges[1]
            .unresolved_reason
            .as_deref()
            .is_some_and(|reason| reason != "resolution-pending")
    );
}

#[test]
fn conditions_follow_importer_format_and_dynamic_import_syntax() {
    let (_, reports) = resolve(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"module":"NodeNext"}}"#,
        ),
        (
            "package.json",
            r##"{"name":"self","type":"module","exports":{"import":"./esm.ts","require":"./cjs.ts"},"imports":{"#value":"./esm.ts"}}"##,
        ),
        ("index.ts", "import { value } from 'self'; import '#value';"),
        ("index.cts", "import { value } from 'self'; import('self');"),
        ("esm.ts", "export const value = 1;"),
        ("cjs.ts", "export const value = 2;"),
    ]);
    let edges = &reports[0].edges;
    assert_eq!(
        edges.iter().map(|edge| edge.to_file).collect::<Vec<_>>(),
        vec![Some(5), Some(5), Some(6), Some(5)]
    );
    assert!(
        edges
            .iter()
            .all(|edge| edge.resolved && edge.unresolved_reason.is_none())
    );
}

#[test]
fn unmatched_export_condition_does_not_fall_back_to_main() {
    let (_, reports) = resolve(&[
        (
            "package.json",
            r#"{"name":"self","exports":{"browser":"./leaf.ts"},"main":"./leaf.ts"}"#,
        ),
        ("index.ts", "import 'self';"),
        ("leaf.ts", "export const value = 1;"),
    ]);
    let edge = &reports[0].edges[0];
    assert!(!edge.resolved && edge.to_file.is_none() && edge.to_external.is_none());
    assert!(
        edge.unresolved_reason
            .as_deref()
            .expect("reason")
            .contains("package-path-not-exported")
    );
}

#[test]
fn base_url_lookup_is_removed_from_ts6_but_still_anchors_paths() {
    for version in ["5.9.3", "6.0.0", "7.0.2"] {
        let package = format!(r#"{{"devDependencies":{{"typescript":"{version}"}}}}"#);
        let (_, reports) = resolve(&[
            ("package.json", &package),
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"baseUrl":"src","paths":{"@leaf":["leaf"]}}}"#,
            ),
            ("index.ts", "import '@leaf'; import 'leaf';"),
            ("src/leaf.ts", "export const value = 1;"),
        ]);
        assert_eq!(reports[0].edges[0].to_file, Some(4), "{version}");
        assert_eq!(
            reports[0].edges[1].resolved,
            version.starts_with('5'),
            "{version}"
        );
    }
}

#[test]
fn declared_ts5_ranges_preserve_base_url_lookup() {
    ts5_ranges_preserve_base_url_lookup(false);
}

#[test]
fn installed_ts5_ranges_preserve_base_url_lookup() {
    ts5_ranges_preserve_base_url_lookup(true);
}

fn ts5_ranges_preserve_base_url_lookup(installed: bool) {
    for version in [
        ">=5.0.0", ">5", "<=5.6.0", "<5.9.0", "=5.6.0", "v5.6.0", "^5.6.0", "~5.6.0",
    ] {
        let (path, package) = if installed {
            (
                "node_modules/typescript/package.json",
                format!(r#"{{"version":"{version}"}}"#),
            )
        } else {
            (
                "package.json",
                format!(r#"{{"devDependencies":{{"typescript":"{version}"}}}}"#),
            )
        };
        let (_, reports) = resolve(&[
            (path, &package),
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"baseUrl":"src","paths":{"@leaf":["leaf"]}}}"#,
            ),
            ("index.ts", "import '@leaf'; import 'leaf';"),
            ("src/leaf.ts", "export const value = 1;"),
        ]);
        for edge in &reports[0].edges {
            assert_eq!(edge.to_file, Some(4), "{version}: {edge:?}");
        }
    }
}

#[test]
fn project_references_select_the_referenced_config_not_the_solution() {
    let (_, reports) = resolve(&[
        (
            "tsconfig.json",
            r#"{"files":[],"references":[{"path":"./app"}]}"#,
        ),
        (
            "app/tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@leaf":["../lib/leaf"]}},"include":["*.ts"]}"#,
        ),
        ("app/index.ts", "import { value } from '@leaf';"),
        ("lib/leaf.ts", "export const value = 1;"),
    ]);
    assert_eq!(
        reports[0].edges[0].to_file,
        Some(4),
        "{:?}",
        reports[0].edges
    );
}

#[test]
fn node_builtins_are_external_and_missing_packages_are_unresolved() {
    let (_, reports) = resolve(&[("index.ts", "import 'node:fs'; import 'missing-package';")]);
    assert_eq!(reports[0].edges[0].to_external.as_deref(), Some("node:fs"));
    assert!(reports[0].edges[0].resolved);
    assert!(!reports[0].edges[1].resolved);
}

#[test]
fn star_reexports_bind_consumers_and_materialize_module_cycles() {
    let (inventory, reports) = resolve(&[
        (
            "index.ts",
            "import { value } from './barrel'; export const start = value;",
        ),
        ("barrel.ts", "export * from './leaf';"),
        (
            "leaf.ts",
            "import { start } from './index'; export const value = start; export default 0;",
        ),
    ]);
    assert!(
        reports[0].edges[0].to_symbol.is_some(),
        "star export binding is observed"
    );
    let directory = tempfile::tempdir().expect("model");
    let path = directory.path().join("model.sqlite");
    warrant_model::build_model(&path, warrant_lang_ts::module_rows(&inventory), reports)
        .expect("store");
    let store = warrant_model::query::QueryStore::open(path).expect("open");
    let query = |sql| {
        store
            .sql(sql, warrant_model::query::QueryLimits::default())
            .expect("query")
            .rows
    };
    assert_eq!(
        query("SELECT COUNT(*) FROM module_edges"),
        vec![vec![serde_json::json!(3)]]
    );
    assert_eq!(
        query("SELECT COUNT(*) FROM module_cycles"),
        vec![vec![serde_json::json!(3)]]
    );
    assert_eq!(
        query(
            "SELECT consumer.path, via_reexport_chain FROM symbol_consumers JOIN symbols ON symbols.id = symbol_id JOIN files ON files.id = symbols.file_id JOIN files AS consumer ON consumer.id = consumer_file WHERE files.path = 'leaf.ts' AND symbols.export_name = 'value' ORDER BY consumer.path"
        ),
        vec![
            vec![serde_json::json!("barrel.ts"), serde_json::json!(1)],
            vec![serde_json::json!("index.ts"), serde_json::json!(1)]
        ]
    );
    assert!(
        query("SELECT id FROM symbols WHERE file_id = 2 AND export_name = 'default'").is_empty()
    );
}

#[test]
fn conflicting_star_exports_do_not_invent_a_binding() {
    let (_, reports) = resolve(&[
        ("index.ts", "import { value } from './barrel';"),
        ("barrel.ts", "export * from './a'; export * from './b';"),
        ("a.ts", "export const value = 1;"),
        ("b.ts", "export const value = 2;"),
    ]);
    assert!(reports[0].edges[0].to_symbol.is_none());
    assert!(
        reports[0]
            .unsupported
            .iter()
            .any(|row| row.reason == "ambiguous-star-export")
    );
}

#[test]
fn captured_dependency_packages_are_external_but_uncaptured_packages_are_not_guessed() {
    let (_, reports) = resolve(&[
        ("index.ts", "import 'dep'; import 'absent';"),
        (
            "node_modules/dep/package.json",
            r#"{"name":"dep","exports":{"default":"./index.js"}}"#,
        ),
        ("node_modules/dep/index.js", "module.exports = 1;"),
    ]);
    assert_eq!(reports[0].edges[0].to_external.as_deref(), Some("dep"));
    assert!(reports[0].edges[0].resolved && reports[0].edges[0].to_file.is_none());
    assert_eq!(
        reports[0].edges[1].unresolved_reason.as_deref(),
        Some("module-not-found")
    );
}

#[test]
fn inherited_paths_use_the_defining_config_directory() {
    let (_, reports) = resolve(&[
        (
            "tsconfig.json",
            "{ // comment\n\"extends\": \"./config/base.json\", }",
        ),
        (
            "config/base.json",
            r#"{"compilerOptions":{"baseUrl":"../lib","paths":{"@leaf":["leaf"]}}}"#,
        ),
        ("index.ts", "import '@leaf'; import 'leaf';"),
        ("lib/leaf.ts", "export const value = 1;"),
    ]);
    assert_eq!(reports[0].edges[0].to_file, Some(4));
    assert_eq!(
        reports[0].edges[1].unresolved_reason.as_deref(),
        Some("module-not-found")
    );
}

#[test]
fn invalid_configs_preserve_each_unresolved_edge() {
    let (_, reports) = resolve(&[
        ("tsconfig.json", "{broken"),
        ("index.ts", "import './leaf'; import './missing';"),
        ("leaf.ts", "export const value = 1;"),
    ]);
    assert_eq!(reports[0].edges.len(), 2);
    assert!(reports[0].edges.iter().all(|edge| {
        !edge.resolved
            && edge
                .unresolved_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("resolution-failed:"))
    }));
}

#[test]
fn namespace_imports_observe_all_exported_bindings() {
    let (_, reports) = resolve(&[
        (
            "index.ts",
            "import * as values from './leaf'; values.value;",
        ),
        ("leaf.ts", "export const value = 1; export const other = 2;"),
    ]);
    let report = &reports[0];
    let names: std::collections::BTreeSet<_> = report
        .edges
        .iter()
        .filter_map(|edge| {
            report
                .symbols
                .iter()
                .find(|symbol| Some(symbol.id) == edge.to_symbol)
                .and_then(|symbol| symbol.export_name.as_deref())
        })
        .collect();
    assert_eq!(names, std::collections::BTreeSet::from(["value", "other"]));
}

#[test]
fn exported_import_bindings_keep_the_original_symbol_consumer_chain() {
    let (inventory, reports) = resolve(&[
        ("index.ts", "import { renamed } from './middle';"),
        (
            "middle.ts",
            "import { value } from './leaf'; export { value as renamed };",
        ),
        ("leaf.ts", "export const value = 1;"),
    ]);
    let directory = tempfile::tempdir().expect("model");
    let path = directory.path().join("model.sqlite");
    warrant_model::build_model(&path, warrant_lang_ts::module_rows(&inventory), reports)
        .expect("store");
    let store = warrant_model::query::QueryStore::open(path).expect("open");
    let rows = store.sql("SELECT COUNT(*) FROM symbol_consumers JOIN symbols ON symbols.id = symbol_id JOIN files ON files.id = consumer_file WHERE symbols.file_id = 3 AND files.path = 'index.ts' AND via_reexport_chain = 1", warrant_model::query::QueryLimits::default()).expect("query").rows;
    assert_eq!(rows, vec![vec![serde_json::json!(1)]]);
}
