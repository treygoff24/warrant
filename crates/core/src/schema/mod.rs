//! Owns JSON Schema generation and the document registry; it must not know CLI output.

use schemars::Schema;
use thiserror::Error;

/// Preserve schemars' keyword order and sort other keys regardless of serde_json features.
pub fn canonical_bytes(schema: &Schema) -> Result<Vec<u8>, serde_json::Error> {
    let mut value = serde_json::to_value(schema)?;
    value.sort_all_objects();
    // Retain schemars' presentation order so existing schema bytes do not change.
    let schema: Schema = serde_json::from_value(value)?;
    let mut bytes = serde_json::to_vec_pretty(&schema)?;
    bytes.push(b'\n');
    Ok(bytes)
}

mod capabilities;
mod census;
mod commands;
mod context;
mod diff;
mod effective_policy;
mod error;
mod evidence;
mod finding;
mod hook;
mod instruments;
mod inventory;
mod manifest;
mod map;
mod obligations;
mod policy;
mod profile;
mod propose;
mod receipt;
mod ruling;
mod snapshot;
mod test_receipt;
mod verdict;
mod verify;

/// Schema generation failed because the document's producing task has not landed.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SchemaError {
    #[error("schema `{0}` is not implemented")]
    NotImplemented(&'static str),
}

/// One stable entry in the emitted-document registry.
#[derive(Clone, Copy)]
pub struct DocumentSchema {
    pub name: &'static str,
    generate: fn() -> Result<Schema, SchemaError>,
}

impl DocumentSchema {
    pub fn generate(self) -> Result<Schema, SchemaError> {
        (self.generate)()
    }

    pub fn implemented(self) -> bool {
        self.generate().is_ok()
    }
}

/// Every v1 emitted document, in stable marker order.
pub const DOCUMENTS: &[DocumentSchema] = &[
    document("warrant.snapshot", snapshot::schema),
    document("warrant.inventory", inventory::schema),
    document("warrant.capabilities", capabilities::schema),
    document("warrant.commands", commands::schema),
    document("warrant.manifest", manifest::schema),
    document("warrant.policy", policy::schema),
    document("warrant.effective-policy", effective_policy::schema),
    document("warrant.obligations", obligations::schema),
    document("warrant.finding", finding::schema),
    document("warrant.verdict", verdict::schema),
    document("warrant.context", context::schema),
    document("warrant.propose", propose::schema),
    document("warrant.map", map::schema),
    document("warrant.census", census::schema),
    document("warrant.test-receipt", test_receipt::schema),
    document("warrant.evidence", evidence::schema),
    document("warrant.instruments", instruments::schema),
    document("warrant.ruling", ruling::schema),
    document("warrant.receipt", receipt::schema),
    document("warrant.verify", verify::schema),
    document("warrant.diff", diff::schema),
    document("warrant.error", error::schema),
    document("warrant.hook", hook::schema),
    document("warrant.profile", profile::schema),
];

const fn document(
    name: &'static str,
    generate: fn() -> Result<Schema, SchemaError>,
) -> DocumentSchema {
    DocumentSchema { name, generate }
}

/// Generate one document schema by its stable name.
pub fn generate(name: &str) -> Result<Schema, SchemaError> {
    DOCUMENTS
        .iter()
        .copied()
        .find(|document| document.name == name)
        .map_or(Err(SchemaError::NotImplemented("unknown")), |document| {
            document.generate()
        })
}
