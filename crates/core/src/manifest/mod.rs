//! Owns `warrant.yaml` parsing and defaults; it must not know compiled policy meaning.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::nouns::InventoryClass;

const MANIFEST_SCHEMA: &str = "warrant.manifest/1";

/// A typed manifest parse failure.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ManifestError {
    #[error("unknown manifest field `{field}`")]
    UnknownField { field: String },
    #[error("unknown inventory class `{class}` in `{field}`")]
    UnknownClass { field: String, class: String },
    #[error("invalid manifest: {reason}")]
    Invalid { reason: String },
}

/// Repository-level Warrant configuration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WarrantManifest {
    pub schema_version: String,
    #[serde(default)]
    pub integrations: BTreeMap<String, IntegrationConfig>,
    #[serde(default)]
    pub policy: PolicyConfig,
    pub profile: Option<String>,
    #[serde(default)]
    pub inventory: InventoryConfig,
    #[serde(default)]
    pub snapshot: SnapshotConfig,
    #[serde(default)]
    pub gate: GateConfig,
    #[serde(default = "default_instruments")]
    pub instruments: String,
    #[serde(default)]
    pub entrypoints: Vec<EntrypointDeclaration>,
}

impl WarrantManifest {
    /// Parse YAML after fail-closed validation of fields and inventory classes.
    pub fn parse(input: &str) -> Result<Self, ManifestError> {
        let value: Value = serde_saphyr::from_str(input).map_err(invalid)?;
        validate_fields(&value)?;
        validate_classes(&value)?;
        let manifest: Self = serde_saphyr::from_str(input).map_err(invalid)?;
        if manifest.schema_version != MANIFEST_SCHEMA {
            return Err(ManifestError::Invalid {
                reason: format!(
                    "schema_version must be `{MANIFEST_SCHEMA}`, got `{}`",
                    manifest.schema_version
                ),
            });
        }
        Ok(manifest)
    }
}

fn invalid(error: serde_saphyr::Error) -> ManifestError {
    ManifestError::Invalid {
        reason: error.to_string(),
    }
}

/// One language integration selected by the manifest.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationConfig {
    #[serde(default)]
    pub enabled: bool,
    pub tsconfig: Option<String>,
    #[serde(default)]
    pub framework_adapters: Vec<String>,
}

/// Paths containing policy documents.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default = "default_policy_paths")]
    pub paths: Vec<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            paths: default_policy_paths(),
        }
    }
}

/// Whether unknown inventory paths are advisory or blocking.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownTreatment {
    #[default]
    Report,
    Block,
}

impl UnknownTreatment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Block => "block",
        }
    }
}

/// An explicit path-class assignment.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassDeclaration {
    pub class: InventoryClass,
    pub files: Vec<String>,
    #[serde(default)]
    pub replaces: Option<InventoryClass>,
}

/// A reproducible generated-path declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedDeclaration {
    pub files: Vec<String>,
    pub producer: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub reproducible: bool,
}

/// A vendored-path declaration.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VendoredDeclaration {
    pub files: Vec<String>,
    pub source: String,
    pub version: String,
    #[serde(default = "default_vendored_treatment")]
    pub treatment: String,
}

/// Inventory classification configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryConfig {
    #[serde(default)]
    pub classes: Vec<ClassDeclaration>,
    #[serde(default)]
    pub unknown: UnknownTreatment,
    #[serde(default)]
    pub generated: Vec<GeneratedDeclaration>,
    #[serde(default)]
    pub vendored: Vec<VendoredDeclaration>,
}

/// Snapshot capture limits.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotConfig {
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

/// Gate hints proposed by repository configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateConfig {
    #[serde(default)]
    pub required_evidence: Vec<String>,
    pub trust_root_hint: Option<String>,
}

/// An explicitly declared external entrypoint.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntrypointDeclaration {
    pub kind: String,
    pub path: String,
    pub symbol: Option<String>,
}

fn default_policy_paths() -> Vec<String> {
    vec!["warrant/policy/*.yaml".into()]
}

const fn default_max_file_bytes() -> u64 {
    8 * 1024 * 1024
}

fn default_instruments() -> String {
    "warrant/instruments.lock".into()
}

fn default_vendored_treatment() -> String {
    "excluded-from-contracts".into()
}

fn validate_fields(value: &Value) -> Result<(), ManifestError> {
    let root = object(value, "manifest")?;
    check_keys(
        root,
        "",
        &[
            "schema_version",
            "integrations",
            "policy",
            "profile",
            "inventory",
            "snapshot",
            "gate",
            "instruments",
            "entrypoints",
        ],
    )?;
    if let Some(integrations) = root.get("integrations").and_then(Value::as_object) {
        for (name, config) in integrations {
            check_keys(
                object(config, &format!("integrations.{name}"))?,
                &format!("integrations.{name}"),
                &["enabled", "tsconfig", "framework_adapters"],
            )?;
        }
    }
    check_optional_object(root, "policy", &["paths"])?;
    check_optional_object(root, "snapshot", &["max_file_bytes"])?;
    check_optional_object(root, "gate", &["required_evidence", "trust_root_hint"])?;
    if let Some(inventory) = root.get("inventory").and_then(Value::as_object) {
        check_keys(
            inventory,
            "inventory",
            &["classes", "unknown", "generated", "vendored"],
        )?;
        check_object_list(inventory, "classes", &["class", "files", "replaces"])?;
        check_object_list(
            inventory,
            "generated",
            &["files", "producer", "inputs", "reproducible"],
        )?;
        check_object_list(
            inventory,
            "vendored",
            &["files", "source", "version", "treatment"],
        )?;
    }
    if let Some(entrypoints) = root.get("entrypoints").and_then(Value::as_array) {
        for (index, entrypoint) in entrypoints.iter().enumerate() {
            let path = format!("entrypoints[{index}]");
            check_keys(
                object(entrypoint, &path)?,
                &path,
                &["kind", "path", "symbol"],
            )?;
        }
    }
    Ok(())
}

fn validate_classes(value: &Value) -> Result<(), ManifestError> {
    let Some(classes) = value
        .get("inventory")
        .and_then(|inventory| inventory.get("classes"))
        .and_then(Value::as_array)
    else {
        return Ok(());
    };
    for (index, declaration) in classes.iter().enumerate() {
        for name in ["class", "replaces"] {
            let Some(class) = declaration.get(name).and_then(Value::as_str) else {
                continue;
            };
            if !matches!(
                class,
                "source"
                    | "test"
                    | "config"
                    | "script"
                    | "migration"
                    | "schema"
                    | "generated"
                    | "vendored"
                    | "asset"
                    | "doc"
                    | "build-output"
                    | "submodule"
                    | "ignored"
                    | "unknown"
                    | "unread"
            ) {
                return Err(ManifestError::UnknownClass {
                    field: format!("inventory.classes[{index}].{name}"),
                    class: class.into(),
                });
            }
        }
    }
    Ok(())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, ManifestError> {
    value.as_object().ok_or_else(|| ManifestError::Invalid {
        reason: format!("`{path}` must be a mapping"),
    })
}

fn check_optional_object(
    root: &Map<String, Value>,
    name: &str,
    allowed: &[&str],
) -> Result<(), ManifestError> {
    if let Some(value) = root.get(name) {
        check_keys(object(value, name)?, name, allowed)?;
    }
    Ok(())
}

fn check_object_list(
    root: &Map<String, Value>,
    name: &str,
    allowed: &[&str],
) -> Result<(), ManifestError> {
    let Some(values) = root.get(name).and_then(Value::as_array) else {
        return Ok(());
    };
    for (index, value) in values.iter().enumerate() {
        let path = format!("inventory.{name}[{index}]");
        check_keys(object(value, &path)?, &path, allowed)?;
    }
    Ok(())
}

fn check_keys(
    object: &Map<String, Value>,
    prefix: &str,
    allowed: &[&str],
) -> Result<(), ManifestError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            let field = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            return Err(ManifestError::UnknownField { field });
        }
    }
    Ok(())
}
