use std::{collections::BTreeSet, fs, path::Path};

use warrant_core::manifest::{ManifestError, WarrantManifest};
use warrant_core::nouns::{
    GeneratedBy, InventoryClass, InventoryDocument, InventorySummary, SnapshotKind,
    SnapshotManifest,
};
use warrant_core::schema::{DOCUMENTS, canonical_bytes};

#[test]
fn manifest_applies_documented_defaults() {
    let manifest = WarrantManifest::parse("schema_version: warrant.manifest/1\n")
        .expect("minimal manifest should parse");

    assert_eq!(manifest.inventory.unknown.as_str(), "report");
    assert_eq!(manifest.snapshot.max_file_bytes, 8 * 1024 * 1024);
    assert_eq!(manifest.policy.paths, ["warrant/policy/*.yaml"]);
    assert_eq!(manifest.instruments, "warrant/instruments.lock");
}

#[test]
fn manifest_rejects_unknown_field_with_its_name() {
    let error = WarrantManifest::parse(
        "schema_version: warrant.manifest/1\nsnapshot:\n  max_file_bytes: 1\n  surprise: true\n",
    )
    .expect_err("unknown fields must fail closed");

    assert_eq!(
        error,
        ManifestError::UnknownField {
            field: "snapshot.surprise".into(),
        }
    );
}

#[test]
fn manifest_rejects_unknown_class_with_its_field() {
    let error = WarrantManifest::parse(
        "schema_version: warrant.manifest/1\ninventory:\n  classes:\n    - class: mystery\n      files: [\"src/**\"]\n",
    )
    .expect_err("unknown inventory classes must fail closed");

    assert_eq!(
        error,
        ManifestError::UnknownClass {
            field: "inventory.classes[0].class".into(),
            class: "mystery".into(),
        }
    );
}

#[test]
fn new_inventory_contract_fields_are_additive_and_pinned() {
    let manifest = WarrantManifest::parse(
        r#"
schema_version: warrant.manifest/1
inventory:
  classes:
    - class: migration
      files: ["src/value.ts"]
      replaces: source
  generated:
    - files: ["generated/**"]
      producer: generate
      inputs: ["schema/api.yaml"]
"#,
    )
    .expect("new manifest fields should parse");
    assert_eq!(
        manifest.inventory.classes[0].replaces,
        Some(InventoryClass::Source)
    );
    assert_eq!(manifest.inventory.generated[0].inputs, ["schema/api.yaml"]);

    let generated: GeneratedBy = serde_json::from_str(
        r#"{"producer":"generate","reproducible":true,"inputs":["schema/api.yaml"]}"#,
    )
    .expect("generated provenance should deserialize");
    assert_eq!(generated.inputs, ["schema/api.yaml"]);
    let old_generated: GeneratedBy =
        serde_json::from_str(r#"{"producer":"generate","reproducible":false}"#)
            .expect("old generated provenance should keep deserializing");
    assert!(old_generated.inputs.is_empty());

    let summary: InventorySummary = serde_json::from_str(
        r#"{
          "files": 0,
          "by_class": {},
          "unread": [],
          "unowned_source": [],
          "unknown": [],
          "ignored_files": 0,
          "submodules": [],
          "unit_aliases": [
            {"unit":".","alias_table":null,"by":"implicit-root-fallback"}
          ],
          "generated_absent": [
            {"declaration":"generated/**","producer":"generate"}
          ]
        }"#,
    )
    .expect("new summary fields should deserialize");
    assert_eq!(summary.unit_aliases[0].unit, ".");
    assert_eq!(summary.unit_aliases[0].alias_table, None);
    assert_eq!(summary.unit_aliases[0].by, "implicit-root-fallback");
    assert_eq!(summary.generated_absent[0].declaration, "generated/**");
    assert_eq!(summary.generated_absent[0].producer, "generate");

    let old_summary: InventorySummary = serde_json::from_str(
        r#"{
          "files": 0,
          "by_class": {},
          "unread": [],
          "unowned_source": [],
          "unknown": [],
          "ignored_files": null,
          "submodules": []
        }"#,
    )
    .expect("old inventory summaries should keep deserializing");
    assert!(old_summary.unit_aliases.is_empty());
    assert!(old_summary.generated_absent.is_empty());

    let old_document: InventoryDocument = serde_json::from_value(serde_json::json!({
        "schema_version": "warrant.inventory/1",
        "entries": [],
        "summary": old_summary
    }))
    .expect("old inventory documents should keep deserializing");
    assert!(!old_document.truncated);
    assert_eq!(old_document.total, 0);
    assert_eq!(old_document.next_cursor, None);

    let schema = warrant_core::schema::generate("warrant.inventory")
        .expect("inventory schema should generate");
    let schema = serde_json::to_value(schema).expect("inventory schema should serialize");
    for field in ["truncated", "total", "next_cursor"] {
        assert!(schema["properties"].get(field).is_some(), "missing {field}");
    }
}

#[test]
fn snapshot_manifest_round_trips() {
    let manifest = SnapshotManifest::new(
        "sha1:repo".into(),
        SnapshotKind::Tree,
        "sha1:tree".into(),
        "2026-09-20T07:00:00Z".into(),
    );
    let encoded = serde_json::to_string(&manifest).expect("snapshot should serialize");
    let decoded: SnapshotManifest =
        serde_json::from_str(&encoded).expect("snapshot should deserialize");

    assert_eq!(decoded, manifest);
    assert_eq!(decoded.schema_version, "warrant.snapshot/1");
    assert_eq!(InventoryClass::Source.as_str(), "source");
}

#[test]
fn implemented_document_schemas_are_stable_and_require_schema_version() {
    let implemented: BTreeSet<_> = DOCUMENTS
        .iter()
        .filter(|document| document.implemented())
        .map(|document| document.name.to_owned())
        .collect();
    for name in [
        "warrant.snapshot",
        "warrant.inventory",
        "warrant.capabilities",
        "warrant.commands",
        "warrant.manifest",
        "warrant.error",
    ] {
        assert!(implemented.contains(name), "{name} must remain implemented");
    }

    let schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let checked_in: BTreeSet<_> = fs::read_dir(&schemas)
        .expect("checked-in schemas directory should exist")
        .map(|entry| {
            entry
                .expect("schema directory entry should be readable")
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .expect("schema filename should be UTF-8")
                .to_owned()
        })
        .collect();
    assert_eq!(
        implemented, checked_in,
        "implemented schemas must match files"
    );

    for document in DOCUMENTS
        .iter()
        .copied()
        .filter(|document| document.implemented())
    {
        let first = canonical_bytes(
            &document
                .generate()
                .expect("implemented schema should generate"),
        )
        .expect("schema should serialize");
        let second = canonical_bytes(
            &document
                .generate()
                .expect("implemented schema should generate"),
        )
        .expect("schema should serialize");
        let schema: serde_json::Value =
            serde_json::from_slice(&first).expect("schema should round-trip as JSON");

        assert_eq!(first, second, "{} was not deterministic", document.name);
        let checked_in = fs::read(schemas.join(format!("{}.json", document.name)))
            .expect("implemented schema file should be readable");
        pretty_assertions::assert_eq!(
            std::str::from_utf8(&first).expect("generated JSON should be UTF-8"),
            std::str::from_utf8(&checked_in).expect("checked-in JSON should be UTF-8"),
            "{} differs from its checked-in bytes",
            document.name
        );
        assert!(
            schema["required"]
                .as_array()
                .expect("document schema should have required fields")
                .iter()
                .any(|field| field == "schema_version"),
            "{} did not require schema_version",
            document.name
        );
        if document.name == "warrant.commands" {
            assert_eq!(schema["additionalProperties"], false);
        }
    }
}
