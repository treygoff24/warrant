use std::{fs, path::Path};

use serde_json::json;
use warrant_core::nouns::InventoryEntry;
use warrant_lang_rust::{Selection, analyze, capture};
use warrant_model::{
    ModelBuilder,
    query::{QueryLimits, QueryStore},
};

fn inventory(root: &Path) -> Vec<InventoryEntry> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<InventoryEntry>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, entries);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "rs" || ext == "toml")
            {
                entries.push(serde_json::from_value(json!({
                    "path": path.strip_prefix(root).unwrap().to_str().unwrap(),
                    "blob": "fixture-blob", "class": if path.extension().unwrap() == "rs" { "source" } else { "config" },
                    "language": if path.extension().unwrap() == "rs" { Some("rust") } else { None },
                    "unit": null, "module": null, "by": "fixture", "reason": "fixture",
                    "unread": null, "generated_by": null, "vendored_from": null
                })).unwrap());
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

fn selection() -> Selection {
    Selection {
        target: "x86_64-unknown-linux-gnu".into(),
        features: Vec::new(),
        no_default_features: true,
        all_features: false,
    }
}

#[test]
fn cargo_resolved_edges_are_queryable_with_their_native_basis() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let input = capture(&root, &selection()).unwrap();
    let reports = analyze(&input, &inventory(&root)).unwrap();
    assert_eq!(reports.len(), 4);
    let app = reports.iter().find(|r| r.unit.root == "app").unwrap();
    assert_eq!(app.edges.len(), 1);
    assert!(app.symbols.is_empty());
    assert!(app.references.is_empty());
    assert_eq!(app.capability_report.json["symbol_level"], "none");
    assert_eq!(app.capability_report.json["resolution_authority"], "native");
    assert_eq!(
        app.capability_report.json["supports"],
        json!(["package-dependency"])
    );
    assert!(
        app.unsupported
            .iter()
            .any(|r| r.construct == "rust-internal-references")
    );
    assert!(
        !app.entrypoints
            .iter()
            .any(|r| r.declared_by.as_deref() == Some("extra-bin"))
    );

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("model.sqlite");
    let mut builder = ModelBuilder::create(&db).unwrap();
    for report in &reports {
        builder.write_report(report).unwrap();
    }
    builder.finish().unwrap();
    let query = QueryStore::open(&db).unwrap();
    let edges = query.sql("SELECT f.path, t.path, e.kind, e.basis, e.resolved FROM edges e JOIN files f ON f.id=e.from_file JOIN files t ON t.id=e.to_file", QueryLimits::default()).unwrap();
    assert_eq!(
        edges.rows,
        vec![vec![
            json!("app/Cargo.toml"),
            json!("core/Cargo.toml"),
            json!("package-dependency"),
            json!("native:cargo-metadata"),
            json!(1)
        ]]
    );
}

#[test]
fn cargo_selects_platform_features_and_feature_gated_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let inventory = inventory(&root);
    for (target, features, no_default_features, all_features, expected_edges, has_bin) in [
        ("x86_64-unknown-linux-gnu", vec![], true, false, 1, false),
        ("x86_64-pc-windows-msvc", vec![], true, false, 2, false),
        (
            "x86_64-unknown-linux-gnu",
            vec!["app/extras".to_string()],
            true,
            false,
            2,
            true,
        ),
        ("x86_64-unknown-linux-gnu", vec![], false, false, 2, true),
        ("x86_64-pc-windows-msvc", vec![], true, true, 3, true),
    ] {
        let selection = Selection {
            target: target.into(),
            features,
            no_default_features,
            all_features,
        };
        let input = capture(&root, &selection).unwrap();
        let reports = analyze(&input, &inventory).unwrap();
        let app = reports.iter().find(|r| r.unit.root == "app").unwrap();
        assert_eq!(app.edges.len(), expected_edges, "{selection:?}");
        assert_eq!(
            app.entrypoints
                .iter()
                .any(|r| r.declared_by.as_deref() == Some("extra-bin")),
            has_bin,
            "{selection:?}"
        );
        assert!(
            app.capability_report.json["limits"]
                .as_str()
                .unwrap()
                .contains(target)
        );
    }
}

#[test]
fn missing_inventory_inputs_are_refused() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let input = capture(&root, &selection()).unwrap();
    let mut entries = inventory(&root);
    entries.retain(|entry| entry.path != "core/Cargo.toml");
    assert!(
        analyze(&input, &entries)
            .unwrap_err()
            .to_string()
            .contains("absent from inventory")
    );
    let mut entries = inventory(&root);
    entries
        .iter_mut()
        .find(|e| e.path == "app/src/lib.rs")
        .unwrap()
        .unread = Some("read-failed".into());
    assert!(
        analyze(&input, &entries)
            .unwrap_err()
            .to_string()
            .contains("unread")
    );
    let mut entries = inventory(&root);
    entries
        .iter_mut()
        .find(|e| e.path == "app/Cargo.toml")
        .unwrap()
        .blob = None;
    assert!(
        analyze(&input, &entries)
            .unwrap_err()
            .to_string()
            .contains("no captured blob")
    );
}

fn copy_fixture(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_fixture(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

#[test]
fn locked_metadata_refuses_missing_lock_and_invalid_selection() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let dir = tempfile::tempdir().unwrap();
    copy_fixture(&source, dir.path());
    let lock = dir.path().join("Cargo.lock");
    assert!(lock.is_file());
    fs::remove_file(&lock).unwrap();
    assert!(
        capture(dir.path(), &selection())
            .unwrap_err()
            .to_string()
            .contains("locked offline Cargo metadata failed")
    );
    assert!(!lock.exists(), "capture must not regenerate the lockfile");
    let mut options = selection();
    options.features.push("app/nonexistent".into());
    assert!(capture(&source, &options).is_err());
    options.target.clear();
    assert!(
        capture(&source, &options)
            .unwrap_err()
            .to_string()
            .contains("target selection is required")
    );
}

fn model_digest(reports: &[warrant_model::IntegrationReport]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let mut model = ModelBuilder::create(dir.path().join("model.sqlite")).unwrap();
    for report in reports {
        model.write_report(report).unwrap();
    }
    model.finish().unwrap()
}

#[test]
fn equivalent_copies_produce_the_same_model_digest() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let dir = tempfile::tempdir().unwrap();
    copy_fixture(&source, dir.path());
    let first = capture(&source, &selection()).unwrap();
    let second = capture(dir.path(), &selection()).unwrap();
    assert_eq!(
        model_digest(&analyze(&first, &inventory(&source)).unwrap()),
        model_digest(&analyze(&second, &inventory(dir.path())).unwrap())
    );
    let mut options = selection();
    options.target = "x86_64-pc-windows-msvc".into();
    let windows = capture(&source, &options).unwrap();
    assert_ne!(
        model_digest(&analyze(&first, &inventory(&source)).unwrap()),
        model_digest(&analyze(&windows, &inventory(&source)).unwrap())
    );
}

#[test]
fn warrants_twelve_units_and_section_3_1_edges_are_queryable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let input = capture(root, &selection()).unwrap();
    let mut entries = inventory(&root.join("crates"));
    for entry in &mut entries {
        entry.path = format!("crates/{}", entry.path);
    }
    let reports = analyze(&input, &entries).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("model.sqlite");
    let mut builder = ModelBuilder::create(&db).unwrap();
    for report in &reports {
        builder.write_report(report).unwrap();
    }
    builder.finish().unwrap();
    let query = QueryStore::open(&db).unwrap();
    let units = query
        .sql(
            "SELECT root FROM units ORDER BY root",
            QueryLimits::default(),
        )
        .unwrap();
    let names = [
        "authority",
        "census",
        "cli",
        "core",
        "evidence",
        "inventory",
        "judgment",
        "lang-rust",
        "lang-ts",
        "model",
        "render",
        "snapshot",
    ];
    assert_eq!(
        units.rows,
        names
            .iter()
            .map(|name| vec![json!(format!("crates/{name}"))])
            .collect::<Vec<_>>()
    );
    let edges = query.sql("SELECT f.path, t.path FROM edges e JOIN files f ON f.id=e.from_file JOIN files t ON t.id=e.to_file ORDER BY f.path, t.path", QueryLimits::default()).unwrap();
    let mut expected = Vec::new();
    // The spec, rather than the product's observed graph, determines these edges.
    for name in [
        "authority",
        "census",
        "evidence",
        "inventory",
        "judgment",
        "model",
        "render",
        "snapshot",
    ] {
        expected.push(vec![
            json!(format!("crates/{name}/Cargo.toml")),
            json!("crates/core/Cargo.toml"),
        ]);
    }
    for name in ["lang-rust", "lang-ts"] {
        for dependency in ["core", "model"] {
            expected.push(vec![
                json!(format!("crates/{name}/Cargo.toml")),
                json!(format!("crates/{dependency}/Cargo.toml")),
            ]);
        }
    }
    for dependency in names.iter().filter(|name| **name != "cli") {
        expected.push(vec![
            json!("crates/cli/Cargo.toml"),
            json!(format!("crates/{dependency}/Cargo.toml")),
        ]);
    }
    expected.sort_by_key(|row| {
        (
            row[0].as_str().unwrap().to_owned(),
            row[1].as_str().unwrap().to_owned(),
        )
    });
    assert_eq!(edges.rows, expected);
    let external = query.sql("SELECT COUNT(*) FROM edges e JOIN files f ON f.id=e.from_file WHERE f.path='crates/lang-rust/Cargo.toml' AND e.to_external = 'registry+https://github.com/rust-lang/crates.io-index#cargo_metadata@0.23.1' AND e.resolved=1 AND e.basis='native:cargo-metadata'", QueryLimits::default()).unwrap();
    assert_eq!(external.rows, vec![vec![json!(1)]]);
}

#[test]
fn build_and_dev_package_edges_survive_target_filtering() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let dir = tempfile::tempdir().unwrap();
    copy_fixture(&source, dir.path());
    let manifest = dir.path().join("app/Cargo.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[build-dependencies]\nextra = { path = \"../extra\" }\n[dev-dependencies]\nwindows = { path = \"../windows\" }\n");
    fs::write(manifest, text).unwrap();
    let input = capture(dir.path(), &selection()).unwrap();
    let reports = analyze(&input, &inventory(dir.path())).unwrap();
    let app = reports.iter().find(|r| r.unit.root == "app").unwrap();
    assert_eq!(app.edges.len(), 3);
    assert!(
        app.edges
            .iter()
            .all(|e| e.resolved && e.basis == "native:cargo-metadata")
    );
}

#[test]
fn cargo_targets_need_not_have_a_rust_extension() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace");
    let dir = tempfile::tempdir().unwrap();
    copy_fixture(&source, dir.path());
    let manifest = dir.path().join("app/Cargo.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[lib]\npath = \"src/code.txt\"\n");
    fs::write(manifest, text).unwrap();
    fs::write(dir.path().join("app/src/code.txt"), "pub fn code() {}\n").unwrap();
    let mut entries = inventory(dir.path());
    let mut target = entries
        .iter()
        .find(|e| e.path == "app/src/lib.rs")
        .unwrap()
        .clone();
    target.path = "app/src/code.txt".into();
    entries.push(target);
    let input = capture(dir.path(), &selection()).unwrap();
    let reports = analyze(&input, &entries).unwrap();
    let app = reports.iter().find(|r| r.unit.root == "app").unwrap();
    let file = app
        .files
        .iter()
        .find(|f| f.path == "app/src/code.txt")
        .unwrap();
    assert!(
        app.entrypoints
            .iter()
            .any(|e| e.file_id == file.id && e.kind == "cargo-target:lib")
    );
    assert!(
        app.unsupported
            .iter()
            .any(|e| e.file_id == file.id && e.construct == "rust-internal-references")
    );
}
