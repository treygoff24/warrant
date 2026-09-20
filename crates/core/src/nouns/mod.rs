//! Owns shared emitted-document nouns; it must not know evaluation or storage behavior.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The exact source kind represented by a snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotKind {
    Commit,
    Index,
    Worktree,
    Tree,
}

/// How snapshot bytes were captured.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub kind: String,
    pub manifest_digest: Option<String>,
}

/// Counts and limits deliberately excluded from a snapshot.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotExclusions {
    pub ignored_files: Option<u64>,
    pub ignored_count_reason: Option<String>,
    pub submodules: u64,
    pub oversize: u64,
}

/// The exact source identity used by all downstream artifacts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotManifest {
    pub schema_version: String,
    pub repo: String,
    pub kind: SnapshotKind,
    pub tree: String,
    pub object_format: String,
    pub commit: Option<String>,
    pub capture: Capture,
    pub excluded: SnapshotExclusions,
    pub taken_at: String,
}

impl SnapshotManifest {
    pub fn new(repo: String, kind: SnapshotKind, tree: String, taken_at: String) -> Self {
        Self {
            schema_version: "warrant.snapshot/1".into(),
            repo,
            kind,
            tree,
            object_format: "sha1".into(),
            commit: None,
            capture: Capture {
                kind: "supplied-tree".into(),
                manifest_digest: None,
            },
            excluded: SnapshotExclusions::default(),
            taken_at,
        }
    }
}

/// The exclusive classification assigned to an inventory path.
#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum InventoryClass {
    Source,
    Test,
    Config,
    Script,
    Migration,
    Schema,
    Generated,
    Vendored,
    Asset,
    Doc,
    BuildOutput,
    Submodule,
    Ignored,
    Unknown,
    Unread,
}

impl InventoryClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Test => "test",
            Self::Config => "config",
            Self::Script => "script",
            Self::Migration => "migration",
            Self::Schema => "schema",
            Self::Generated => "generated",
            Self::Vendored => "vendored",
            Self::Asset => "asset",
            Self::Doc => "doc",
            Self::BuildOutput => "build-output",
            Self::Submodule => "submodule",
            Self::Ignored => "ignored",
            Self::Unknown => "unknown",
            Self::Unread => "unread",
        }
    }
}

/// A runtime-visible entrypoint and the basis that established it.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entrypoint {
    pub kind: String,
    pub basis: String,
    pub by: String,
}

/// Producer provenance for generated content.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedBy {
    pub producer: String,
    pub reproducible: bool,
}

/// Source provenance for vendored content.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VendoredFrom {
    pub source: String,
    pub version: String,
    pub treatment: String,
}

/// One classified path in the inventory denominator.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryEntry {
    pub path: String,
    pub blob: Option<String>,
    pub class: InventoryClass,
    pub language: Option<String>,
    pub unit: Option<String>,
    pub module: Option<String>,
    pub by: String,
    pub reason: String,
    #[serde(default)]
    pub entrypoints: Vec<Entrypoint>,
    pub unread: Option<String>,
    pub generated_by: Option<GeneratedBy>,
    pub vendored_from: Option<VendoredFrom>,
}

/// A path Warrant could not read and the reason why.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnreadPath {
    pub path: String,
    pub reason: String,
}

/// A submodule recorded but not descended.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Submodule {
    pub path: String,
    pub commit: String,
}

/// Completeness accounting for the entire inventory.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySummary {
    pub files: u64,
    pub by_class: BTreeMap<String, u64>,
    pub unread: Vec<UnreadPath>,
    pub unowned_source: Vec<String>,
    pub unknown: Vec<String>,
    pub ignored_files: Option<u64>,
    pub submodules: Vec<Submodule>,
}

/// The complete classified inventory document.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryDocument {
    pub schema_version: String,
    pub entries: Vec<InventoryEntry>,
    pub summary: InventorySummary,
}

/// One unsupported construct and its explicit treatment.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedCapability {
    pub construct: String,
    pub treatment: String,
}

/// Claims an integration can and cannot support.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityReport {
    pub schema_version: String,
    pub integration: String,
    pub version: String,
    pub instruments: BTreeMap<String, String>,
    pub resolution_oracle: Option<String>,
    pub compiler_reference_instrument: Option<String>,
    pub resolution_authority: String,
    pub resolution_modes_qualified: Vec<String>,
    pub symbol_level: String,
    pub type_only_distinction: bool,
    pub supports: Vec<String>,
    pub unsupported: Vec<UnsupportedCapability>,
    pub limits: String,
}

/// A bounded source location relevant to an error.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub path: String,
    pub line: Option<u64>,
    pub column: Option<u64>,
}

/// A structured command error.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorDocument {
    pub schema_version: String,
    pub code: String,
    pub reason: String,
    pub locations: Vec<Location>,
    pub next_diagnostic: Option<String>,
}

/// Identity of the exact Warrant executable that produced an artifact.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    pub name: String,
    pub version: String,
    pub git_sha: Option<String>,
    pub target: String,
    pub binary_digest: String,
}
