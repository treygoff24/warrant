use std::collections::BTreeSet;

use serde_json::json;
use tempfile::TempDir;
use warrant_model::{
    CapabilityReportRow, EdgeRow, EffectRow, EntrypointRow, FileRow, IntegrationReport,
    ModelBuilder, ModuleRow, ReferenceRow, SymbolRow, UnitRow, UnsupportedRow,
    query::{QueryLimits, QueryStore},
};

fn unit() -> UnitRow {
    UnitRow {
        id: 1,
        integration: "typescript".into(),
        root: "apps/web".into(),
        config_path: Some("apps/web/tsconfig.json".into()),
        kind: "package".into(),
    }
}

fn report(file_rows: Vec<FileRow>) -> IntegrationReport {
    IntegrationReport {
        unit: unit(),
        files: file_rows,
        symbols: vec![SymbolRow {
            id: 20,
            file_id: 10,
            name: "run".into(),
            kind: "function".into(),
            exported: true,
            export_name: Some("run".into()),
            visibility: "public".into(),
            span_start: 0,
            span_end: 3,
        }],
        edges: vec![EdgeRow {
            id: 30,
            kind: "import".into(),
            from_file: 10,
            from_symbol: Some(20),
            to_file: Some(11),
            to_symbol: Some(21),
            to_external: None,
            resolved: true,
            unresolved_reason: None,
            type_only: false,
            span_start: 4,
            span_end: 10,
            basis: "observed-static".into(),
        }],
        references: vec![ReferenceRow {
            id: 40,
            file_id: 10,
            symbol_id: 20,
            span_start: 11,
            span_end: 14,
        }],
        entrypoints: vec![EntrypointRow {
            id: 50,
            file_id: 10,
            symbol_id: Some(20),
            kind: "package-export".into(),
            basis: "observed-static".into(),
            declared_by: None,
        }],
        effects: vec![EffectRow {
            id: 60,
            name: "network".into(),
            symbol_id: 20,
            declared_by: "contract:web".into(),
        }],
        unsupported: vec![UnsupportedRow {
            id: 70,
            file_id: 10,
            construct: "computed-import".into(),
            span_start: 15,
            span_end: 20,
            reason: "non-literal specifier".into(),
        }],
        capability_report: CapabilityReportRow {
            integration: "typescript".into(),
            json: json!({"symbol_level": "binding"}),
        },
    }
}

fn files() -> Vec<FileRow> {
    vec![
        FileRow {
            id: 10,
            path: "apps/web/src/a.ts".into(),
            blob: "sha1:a".into(),
            class: "source".into(),
            language: Some("typescript".into()),
            unit_id: 1,
            module_id: Some(100),
        },
        FileRow {
            id: 11,
            path: "apps/web/src/b.ts".into(),
            blob: "sha1:b".into(),
            class: "source".into(),
            language: Some("typescript".into()),
            unit_id: 1,
            module_id: Some(101),
        },
        FileRow {
            id: 12,
            path: "apps/web/src/orphan.ts".into(),
            blob: "sha1:c".into(),
            class: "source".into(),
            language: Some("typescript".into()),
            unit_id: 1,
            module_id: None,
        },
    ]
}

fn build(
    temp: &TempDir,
    name: &str,
    reverse: bool,
    timestamp: &str,
) -> (std::path::PathBuf, String) {
    let path = temp.path().join(name);
    let mut builder = ModelBuilder::create(&path).expect("create model");
    builder
        .write_meta("taken_at", timestamp)
        .expect("write timestamp");
    let manifest = json!({"tree": "sha1:tree", "taken_at": timestamp}).to_string();
    builder
        .write_meta("snapshot_manifest", &manifest)
        .expect("write manifest");

    let mut modules = vec![
        ModuleRow {
            id: 100,
            name: "web/a".into(),
            declared_by: "contract:a".into(),
            intent: Some("starts work".into()),
            owner: Some("team-a".into()),
        },
        ModuleRow {
            id: 101,
            name: "web/b".into(),
            declared_by: "contract:b".into(),
            intent: Some("finishes work".into()),
            owner: Some("team-b".into()),
        },
    ];
    if reverse {
        modules.reverse();
    }
    for module in modules {
        builder.write_module(&module).expect("write module");
    }

    let mut file_rows = files();
    if reverse {
        file_rows.reverse();
    }
    let target_symbol = SymbolRow {
        id: 21,
        file_id: 11,
        name: "finish".into(),
        kind: "function".into(),
        exported: true,
        export_name: Some("finish".into()),
        visibility: "public".into(),
        span_start: 0,
        span_end: 6,
    };
    let reverse_edge = EdgeRow {
        id: 31,
        kind: "import".into(),
        from_file: 11,
        from_symbol: Some(21),
        to_file: Some(10),
        to_symbol: Some(20),
        to_external: None,
        resolved: true,
        unresolved_reason: None,
        type_only: true,
        span_start: 7,
        span_end: 12,
        basis: "observed-static".into(),
    };
    if reverse {
        builder
            .write_edge(&reverse_edge)
            .expect("write reverse edge");
        builder
            .write_symbol(&target_symbol)
            .expect("write target symbol");
    }
    builder
        .write_report(&report(file_rows))
        .expect("write integration report");
    if !reverse {
        builder
            .write_symbol(&target_symbol)
            .expect("write target symbol");
        builder
            .write_edge(&reverse_edge)
            .expect("write reverse edge");
    }
    let digest = builder.finish().expect("finish model");
    (path, digest)
}

#[test]
fn schema_contains_every_table_and_view() {
    let temp = TempDir::new().expect("temporary directory");
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(&path).expect("open model");
    let result = store
        .sql(
            "SELECT name, type FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
            QueryLimits::default(),
        )
        .expect("query schema");
    let found = result
        .rows
        .iter()
        .map(|row| {
            (
                row[0].as_str().unwrap().to_owned(),
                row[1].as_str().unwrap().to_owned(),
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        ("capability_reports", "table"),
        ("edges", "table"),
        ("effects", "table"),
        ("entrypoints", "table"),
        ("files", "table"),
        ("meta", "table"),
        ("module_cycles", "table"),
        ("module_edges", "table"),
        ("modules", "table"),
        ("references", "table"),
        ("symbol_consumers", "table"),
        ("symbols", "table"),
        ("units", "table"),
        ("unowned_files", "view"),
        ("unsupported", "table"),
    ]
    .into_iter()
    .map(|(name, kind)| (name.to_owned(), kind.to_owned()))
    .collect::<BTreeSet<_>>();
    assert_eq!(found, expected);

    for name in expected.iter().map(|(name, _)| name) {
        let quoted = if name == "references" {
            "\"references\"".to_owned()
        } else {
            name.to_owned()
        };
        store
            .sql(
                &format!("SELECT * FROM {quoted} LIMIT 0"),
                QueryLimits::default(),
            )
            .unwrap_or_else(|error| panic!("{name} was not queryable: {error}"));
    }
}

#[test]
fn materialized_views_contain_edges_consumers_cycles_and_unowned_files() {
    let temp = TempDir::new().expect("temporary directory");
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(&path).expect("open model");

    let module_edges = store
        .sql("SELECT from_module, to_module, edge_count, type_only_count, first_edge_id FROM module_edges ORDER BY from_module", QueryLimits::default())
        .expect("module edges");
    assert_eq!(
        module_edges.rows,
        vec![
            vec![json!(100), json!(101), json!(1), json!(0), json!(30)],
            vec![json!(101), json!(100), json!(1), json!(1), json!(31)]
        ]
    );

    let consumers = store
        .sql("SELECT symbol_id, consumer_file, via_reexport_chain FROM symbol_consumers ORDER BY symbol_id", QueryLimits::default())
        .expect("symbol consumers");
    assert_eq!(
        consumers.rows,
        vec![
            vec![json!(20), json!(11), json!(0)],
            vec![json!(21), json!(10), json!(0)]
        ]
    );

    let cycles = store
        .sql(
            "SELECT cycle_id, module_id, position FROM module_cycles ORDER BY cycle_id, position",
            QueryLimits::default(),
        )
        .expect("module cycles");
    assert_eq!(
        cycles.rows,
        vec![
            vec![json!(1), json!(100), json!(0)],
            vec![json!(1), json!(101), json!(1)]
        ]
    );

    let unowned = store
        .sql("SELECT id, path FROM unowned_files", QueryLimits::default())
        .expect("unowned files");
    assert_eq!(
        unowned.rows,
        vec![vec![json!(12), json!("apps/web/src/orphan.ts")]]
    );
}

#[test]
fn digest_is_stable_across_insertion_order_and_timestamps() {
    let temp = TempDir::new().expect("temporary directory");
    let (_, first) = build(&temp, "first.sqlite", false, "2026-09-24T00:00:00Z");
    let (second_path, second) = build(&temp, "second.sqlite", true, "2026-09-25T00:00:00Z");
    assert_eq!(first, second);

    let store = QueryStore::open(second_path).expect("open model");
    let stored = store
        .sql(
            "SELECT value FROM meta WHERE key = 'model_digest'",
            QueryLimits::default(),
        )
        .expect("stored digest");
    assert_eq!(stored.rows, vec![vec![json!(second)]]);
}

#[test]
fn authorizer_rejects_every_named_write_statement_kind() {
    let temp = TempDir::new().expect("temporary directory");
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(&path).expect("open model");
    let mut failures = Vec::new();
    for sql in [
        "INSERT INTO meta VALUES ('x', 'y')",
        "UPDATE meta SET value = 'y' WHERE key = 'taken_at'",
        "DELETE FROM meta",
        "ATTACH DATABASE ':memory:' AS other",
        "PRAGMA user_version",
        "PRAGMA user_version = 1",
    ] {
        let result = store.sql(sql, QueryLimits::default());
        if !matches!(&result,
            Err(warrant_model::ModelError::Sql(rusqlite::Error::SqliteFailure(error, Some(message))))
                if error.code == rusqlite::ErrorCode::AuthorizationForStatementDenied && message == "not authorized")
        {
            failures.push(format!("{sql}: expected authorizer denial, got {result:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        store
            .sql("SELECT key FROM meta", QueryLimits::default())
            .is_ok()
    );
}

#[test]
fn row_cap_sets_truncated_and_continuation() {
    let temp = TempDir::new().expect("temporary directory");
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(&path).expect("open model");
    let result = store
        .sql(
            "SELECT id FROM files ORDER BY id",
            QueryLimits {
                rows: 2,
                bytes: 4096,
            },
        )
        .expect("bounded query");
    assert_eq!(result.rows, vec![vec![json!(10)], vec![json!(11)]]);
    assert!(result.truncated);
    assert_eq!(result.continuation.as_deref(), Some("rerun with OFFSET 2"));
}

#[test]
fn byte_cap_sets_truncated_and_continuation() {
    let temp = TempDir::new().expect("temporary directory");
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(&path).expect("open model");
    let result = store
        .sql(
            "SELECT path FROM files ORDER BY id",
            QueryLimits {
                rows: 100,
                bytes: 30,
            },
        )
        .expect("bounded query");
    assert_eq!(result.rows.len(), 1);
    assert!(result.truncated);
    assert_eq!(result.continuation.as_deref(), Some("rerun with OFFSET 1"));
}

fn unit_reports(conflicting: bool) -> Vec<IntegrationReport> {
    (1..=2)
        .map(|id| IntegrationReport {
            unit: UnitRow { id, root: format!("package-{id}"), ..unit() },
            files: vec![], symbols: vec![], edges: vec![], references: vec![],
            entrypoints: vec![], effects: vec![], unsupported: vec![],
            capability_report: CapabilityReportRow {
                integration: "typescript".into(),
                json: json!({"symbol_level": if conflicting && id == 2 { "none" } else { "binding" }}),
            },
        })
        .collect()
}

#[test]
fn repeated_capabilities_are_order_independent() {
    let temp = TempDir::new().unwrap();
    let mut digests = Vec::new();
    for reverse in [false, true] {
        let path = temp.path().join(format!("{reverse}.sqlite"));
        let mut reports = unit_reports(false);
        if reverse {
            reports.reverse();
        }
        digests.push(warrant_model::build_model(&path, [], reports).unwrap());
        let store = QueryStore::open(path).unwrap();
        assert_eq!(
            store
                .sql("SELECT COUNT(*) FROM units", QueryLimits::default())
                .unwrap()
                .rows,
            vec![vec![json!(2)]]
        );
        assert_eq!(
            store
                .sql(
                    "SELECT integration, json FROM capability_reports",
                    QueryLimits::default()
                )
                .unwrap()
                .rows,
            vec![vec![
                json!("typescript"),
                json!(r#"{"symbol_level":"binding"}"#)
            ]]
        );
    }
    assert_eq!(digests[0], digests[1]);
}

#[test]
fn conflicting_capabilities_are_rejected_in_both_orders() {
    let temp = TempDir::new().unwrap();
    let results: Vec<_> = [false, true]
        .into_iter()
        .map(|reverse| {
            let mut reports = unit_reports(true);
            if reverse {
                reports.reverse();
            }
            warrant_model::build_model(temp.path().join(format!("{reverse}.sqlite")), [], reports)
        })
        .collect();
    eprintln!("conflicting reports in forward/reverse order: {results:?}");
    for result in results {
        assert!(
            matches!(result, Err(warrant_model::ModelError::Invalid(message))
            if message.contains("conflicting capability report") && message.contains("typescript"))
        );
    }
}

fn meta_digest(key: &str, value: &str) -> String {
    let temp = TempDir::new().unwrap();
    let mut builder = ModelBuilder::create(temp.path().join("meta.sqlite")).unwrap();
    builder.write_meta(key, value).unwrap();
    builder.finish().unwrap()
}

#[test]
fn digest_includes_meta_keys_ending_in_at_without_an_underscore() {
    assert_ne!(meta_digest("format", "one"), meta_digest("format", "two"));
}

#[test]
fn digest_preserves_scalar_meta_bytes() {
    assert_ne!(
        meta_digest("integration_version", "1"),
        meta_digest("integration_version", "1.0")
    );
}

#[test]
fn first_row_exceeding_byte_cap_errors_without_a_continuation() {
    let temp = TempDir::new().unwrap();
    let (path, _) = build(&temp, "model.sqlite", false, "now");
    let store = QueryStore::open(path).unwrap();
    let result = store.sql(
        "SELECT path FROM files WHERE id = 10",
        QueryLimits {
            rows: 100,
            bytes: 1,
        },
    );
    assert!(
        matches!(&result, Err(warrant_model::ModelError::Invalid(message))
        if message.contains("row-too-large") && message.contains("row 1")),
        "{result:?}"
    );
}

#[test]
fn every_table_preserves_written_column_values() {
    let temp = TempDir::new().unwrap();
    let (path, _) = build(&temp, "model.sqlite", false, "2026-09-24T00:00:00Z");
    let store = QueryStore::open(path).unwrap();
    for (sql, expected) in [
        (
            "SELECT * FROM meta WHERE key = 'taken_at'",
            json!([["taken_at", "2026-09-24T00:00:00Z"]]),
        ),
        (
            "SELECT * FROM units",
            json!([[
                1,
                "typescript",
                "apps/web",
                "apps/web/tsconfig.json",
                "package"
            ]]),
        ),
        (
            "SELECT * FROM files WHERE id = 10",
            json!([[
                10,
                "apps/web/src/a.ts",
                "sha1:a",
                "source",
                "typescript",
                1,
                100
            ]]),
        ),
        (
            "SELECT * FROM modules WHERE id = 100",
            json!([[100, "web/a", "contract:a", "starts work", "team-a"]]),
        ),
        (
            "SELECT * FROM symbols WHERE id = 20",
            json!([[20, 10, "run", "function", 1, "run", "public", 0, 3]]),
        ),
        (
            "SELECT * FROM edges WHERE id = 30",
            json!([[
                30,
                "import",
                10,
                20,
                11,
                21,
                null,
                1,
                null,
                0,
                4,
                10,
                "observed-static"
            ]]),
        ),
        (
            "SELECT * FROM \"references\"",
            json!([[40, 10, 20, 11, 14]]),
        ),
        (
            "SELECT * FROM entrypoints",
            json!([[50, 10, 20, "package-export", "observed-static", null]]),
        ),
        (
            "SELECT * FROM effects",
            json!([[60, "network", 20, "contract:web"]]),
        ),
        (
            "SELECT * FROM unsupported",
            json!([[70, 10, "computed-import", 15, 20, "non-literal specifier"]]),
        ),
        (
            "SELECT * FROM capability_reports",
            json!([["typescript", "{\"symbol_level\":\"binding\"}"]]),
        ),
    ] {
        let result = store.sql(sql, QueryLimits::default()).unwrap();
        assert_eq!(json!(result.rows), expected, "{sql}");
    }
}

#[test]
fn consumers_follow_reexport_bindings_and_preserve_any_reexport_route() {
    let temp = TempDir::new().unwrap();
    let mut digests = Vec::new();
    for reverse in [false, true] {
        let path = temp.path().join(format!("{reverse}.sqlite"));
        let mut builder = ModelBuilder::create(&path).unwrap();
        builder.write_unit(&unit()).unwrap();
        for id in 10..=14 {
            builder
                .write_file(&FileRow {
                    id,
                    path: format!("{id}.ts"),
                    module_id: None,
                    ..files()[0].clone()
                })
                .unwrap();
        }
        for (id, file_id, name) in [(20, 10, "origin"), (21, 11, "barrel"), (22, 13, "outer")] {
            builder
                .write_symbol(&SymbolRow {
                    id,
                    file_id,
                    name: name.into(),
                    export_name: Some(name.into()),
                    ..report(vec![]).symbols[0].clone()
                })
                .unwrap();
        }
        let mut edges = vec![
            EdgeRow {
                id: 30,
                kind: "reexport".into(),
                from_file: 11,
                from_symbol: Some(21),
                to_file: Some(10),
                to_symbol: Some(20),
                ..report(vec![]).edges[0].clone()
            },
            EdgeRow {
                id: 31,
                from_file: 12,
                from_symbol: None,
                to_file: Some(11),
                to_symbol: Some(21),
                ..report(vec![]).edges[0].clone()
            },
            EdgeRow {
                id: 32,
                kind: "reexport".into(),
                from_file: 13,
                from_symbol: Some(22),
                to_file: Some(11),
                to_symbol: Some(21),
                ..report(vec![]).edges[0].clone()
            },
            // File 12 has both direct and re-export routes; either re-export route must set the flag.
            EdgeRow {
                id: 33,
                from_file: 12,
                from_symbol: None,
                to_file: Some(10),
                to_symbol: Some(20),
                ..report(vec![]).edges[0].clone()
            },
        ];
        edges.push(EdgeRow {
            id: 34,
            from_file: 14,
            from_symbol: None,
            to_file: Some(13),
            to_symbol: Some(22),
            ..report(vec![]).edges[0].clone()
        });
        if reverse {
            edges.reverse();
        }
        for edge in edges {
            builder.write_edge(&edge).unwrap();
        }
        digests.push(builder.finish().unwrap());
        let store = QueryStore::open(path).unwrap();
        let result = store
            .sql(
                "SELECT * FROM symbol_consumers ORDER BY symbol_id, consumer_file",
                QueryLimits::default(),
            )
            .unwrap();
        assert_eq!(
            json!(result.rows),
            json!([
                [20, 11, 1],
                [20, 12, 1],
                [20, 13, 1],
                [20, 14, 1],
                [21, 12, 0],
                [21, 13, 1],
                [21, 14, 1],
                [22, 14, 0]
            ])
        );
    }
    assert_eq!(digests[0], digests[1]);
}
