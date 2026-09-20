use warrant_core::manifest::{ManifestError, WarrantManifest};
use warrant_core::nouns::{InventoryClass, SnapshotKind, SnapshotManifest};
use warrant_core::schema::DOCUMENTS;

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
    for document in DOCUMENTS
        .iter()
        .copied()
        .filter(|document| document.implemented())
    {
        let first = serde_json::to_vec_pretty(
            &document
                .generate()
                .expect("implemented schema should generate"),
        )
        .expect("schema should serialize");
        let second = serde_json::to_vec_pretty(
            &document
                .generate()
                .expect("implemented schema should generate"),
        )
        .expect("schema should serialize");
        let schema: serde_json::Value =
            serde_json::from_slice(&first).expect("schema should round-trip as JSON");

        assert_eq!(first, second, "{} was not deterministic", document.name);
        assert!(
            schema["required"]
                .as_array()
                .expect("document schema should have required fields")
                .iter()
                .any(|field| field == "schema_version"),
            "{} did not require schema_version",
            document.name
        );
    }
}
