use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const POLICY_SCHEMA: &str = "warrant.policy/1";
pub const EFFECTIVE_POLICY_SCHEMA: &str = "warrant.effective-policy/1";

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub schema_version: String,
    #[serde(default)]
    pub contracts: Vec<PolicyContract>,
    #[serde(default)]
    pub declarations: Declarations,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct PolicyContract {
    pub id: String,
    pub intent: String,
    pub owner: String,
    pub authority: String,
    #[serde(default)]
    pub class: ContractClass,
    pub expires: Option<String>,
    pub claim: Option<Claim>,
    pub requires_capabilities: Option<BTreeSet<Capability>>,
    pub enforcement: Enforcement,
    pub limits: String,
    pub on_violation: Option<Consequence>,
    #[serde(default)]
    pub supersedes: BTreeSet<String>,
    #[serde(default)]
    pub overrides: BTreeSet<String>,
    #[serde(flatten)]
    pub body: ContractBody,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct EffectiveContract {
    pub id: String,
    pub intent: String,
    pub owner: String,
    pub authority: String,
    pub class: ContractClass,
    pub expires: Option<String>,
    pub claim: Claim,
    pub requires_capabilities: BTreeSet<Capability>,
    pub enforcement: Enforcement,
    pub limits: String,
    pub on_violation: Consequence,
    pub supersedes: BTreeSet<String>,
    pub overrides: BTreeSet<String>,
    #[serde(flatten)]
    pub body: ContractBody,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ContractBody {
    Module(ModuleContract),
    Dependency(DependencyContract),
    Interface(InterfaceContract),
    Effect(EffectContract),
    State(StateContract),
    Capability(CapabilityContract),
    Pattern(PatternContract),
    Evidence(EvidenceContract),
    Data(DataContract),
}

impl ContractBody {
    pub const fn kind(&self) -> ContractKind {
        match self {
            Self::Module(_) => ContractKind::Module,
            Self::Dependency(_) => ContractKind::Dependency,
            Self::Interface(_) => ContractKind::Interface,
            Self::Effect(_) => ContractKind::Effect,
            Self::State(_) => ContractKind::State,
            Self::Capability(_) => ContractKind::Capability,
            Self::Pattern(_) => ContractKind::Pattern,
            Self::Evidence(_) => ContractKind::Evidence,
            Self::Data(_) => ContractKind::Data,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, Ord, PartialEq, PartialOrd)]
pub enum ContractKind {
    Module,
    Dependency,
    Interface,
    Effect,
    State,
    Capability,
    Pattern,
    Evidence,
    Data,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractClass {
    #[default]
    Invariant,
    Preference,
    Migration,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Claim {
    DeclaredOwnership,
    ResolvedDependencyBoundary,
    ObservedConsumerBoundary,
    RecognizedWriteBoundary,
    RequiredStructure,
    TestedBehavior,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    ModuleResolution,
    BindingReferences,
    RecognizedCallSites,
    StructuralMatch,
    AuthenticatedTestCaseResults,
    CompilerReferences,
    ControlFlow,
    TaintFlow,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    Static,
    Pattern,
    Evidence,
    Mixed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Consequence {
    Fail,
    Review,
    Note,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleContract {
    pub files: BTreeSet<String>,
    pub interface: ModuleInterface,
    #[serde(default)]
    pub owns: ModuleOwnership,
    #[serde(default)]
    pub dynamic_loads: DynamicLoads,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleInterface {
    pub entry: String,
    #[serde(default)]
    pub exports: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleOwnership {
    #[serde(default)]
    pub responsibilities: BTreeSet<String>,
    #[serde(default)]
    pub stores: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DynamicLoads {
    #[default]
    Forbid,
    DeclaredOnly,
    UnresolvedAllowed,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyContract {
    pub from: ModuleSelector,
    #[serde(default)]
    pub allow: DependencyTargets,
    #[serde(default)]
    pub deny: DependencyTargets,
    #[serde(default)]
    pub default: Option<DependencyDefault>,
    #[serde(default)]
    pub include_type_only: bool,
    #[serde(default)]
    pub declares: Vec<DeclaredEdge>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyDefault {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredEdge {
    pub target: String,
    pub reason: String,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceContract {
    pub module: String,
    #[serde(default)]
    pub only_via_entry: bool,
    #[serde(default)]
    pub consumers: ModuleSelector,
    #[serde(default)]
    pub exports: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectContract {
    pub interface: SymbolSelector,
    pub permitted_callers: ModuleSelector,
    pub required_context: PatternDefinition,
    #[serde(default)]
    pub evidence: Vec<EvidenceRequirement>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateContract {
    pub store: String,
    pub writers: ModuleSelector,
    pub write_sites: PatternDefinition,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityContract {
    pub instances: RegistrySelector,
    pub requires: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySelector {
    pub registry: String,
    #[serde(default)]
    pub may_be_empty: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PatternContract {
    pub scope: SubjectSelector,
    pub rule: Value,
    #[serde(default)]
    pub forbid: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContract {
    pub requires: EvidenceRequirement,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataContract {
    pub sources: SymbolSelector,
    pub permitted_sinks: ModuleSelector,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequirement {
    pub kind: String,
    pub tag: String,
    pub evidence_kind: String,
    pub unit: Option<String>,
    pub scenario: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PatternDefinition {
    pub pattern: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSelector {
    #[serde(default)]
    pub modules: BTreeSet<String>,
    #[serde(default)]
    pub may_be_empty: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyTargets {
    #[serde(default)]
    pub modules: BTreeSet<String>,
    #[serde(default)]
    pub packages: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolSelector {
    pub symbols: BTreeSet<String>,
    #[serde(default)]
    pub may_be_empty: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectSelector {
    #[serde(default)]
    pub modules: BTreeSet<String>,
    #[serde(default)]
    pub packages: BTreeSet<String>,
    #[serde(default)]
    pub symbols: BTreeSet<String>,
    #[serde(default)]
    pub units: BTreeSet<String>,
    #[serde(default)]
    pub stores: BTreeSet<String>,
    #[serde(default)]
    pub may_be_empty: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declarations {
    #[serde(default)]
    pub stores: Vec<StoreDeclaration>,
    #[serde(default)]
    pub registries: Vec<RegistryDeclaration>,
    #[serde(default)]
    pub external_consumers: Vec<ExternalConsumerDeclaration>,
    #[serde(default)]
    pub loaders: Vec<LoaderDeclaration>,
}

impl Declarations {
    pub(crate) fn extend(&mut self, other: Self) {
        self.stores.extend(other.stores);
        self.registries.extend(other.registries);
        self.external_consumers.extend(other.external_consumers);
        self.loaders.extend(other.loaders);
    }

    pub(crate) fn sort(&mut self) {
        self.stores.sort_by(|left, right| left.id.cmp(&right.id));
        self.registries
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.external_consumers
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.loaders.sort_by(|left, right| left.id.cmp(&right.id));
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreDeclaration {
    pub id: String,
    pub kind: String,
    pub defined_in: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryDeclaration {
    pub id: String,
    pub intent: String,
    pub file: String,
    pub rule: Value,
    #[serde(default)]
    pub captures: BTreeMap<String, String>,
    pub entrypoint_kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalConsumerDeclaration {
    pub id: String,
    pub depends_on: SymbolSelector,
    pub compatibility: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoaderDeclaration {
    pub id: String,
    pub file: String,
    pub references: FileSelector,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileSelector {
    pub files: BTreeSet<String>,
    #[serde(default)]
    pub may_be_empty: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectivePolicy {
    pub schema_version: String,
    pub policy_digest: String,
    pub contracts: Vec<EffectiveContract>,
    pub declarations: Declarations,
    pub sources: Vec<PolicySourceRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySourceRecord {
    pub path: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LintIssue {
    pub code: String,
    pub contracts: Vec<String>,
    pub reason: String,
}
