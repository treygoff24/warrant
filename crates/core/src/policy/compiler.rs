use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::types::{
    Capability, Claim, Consequence, ContractBody, ContractClass, ContractKind, Declarations,
    DependencyTargets, EFFECTIVE_POLICY_SCHEMA, EffectiveContract, EffectivePolicy, Enforcement,
    LintIssue, POLICY_SCHEMA, PolicyContract, PolicyDocument, PolicySourceRecord,
};

const EXPIRED_CONTRACT: &str = "expired-contract";

#[derive(Clone, Copy, Debug)]
pub struct PolicySource<'a> {
    pub path: &'a str,
    pub yaml: &'a str,
}

impl<'a> PolicySource<'a> {
    pub const fn new(path: &'a str, yaml: &'a str) -> Self {
        Self { path, yaml }
    }
}

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("{path}: invalid policy YAML: {reason}")]
    Parse { path: String, reason: String },
    #[error("{path}: schema_version must be `{POLICY_SCHEMA}`, got `{actual}`")]
    SchemaVersion { path: String, actual: String },
    #[error("policy lint failed: {0}", format_issues(.issues))]
    Lint { issues: Vec<LintIssue> },
    #[error("could not canonicalize the effective policy: {0}")]
    Canonical(#[from] serde_json::Error),
}

impl PolicyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse { .. } => "invalid-policy",
            Self::SchemaVersion { .. } => "unsupported-policy-version",
            Self::Lint { issues } => match issues.first() {
                Some(issue) if issue.code == "enforcement-unsupported" => "enforcement-unsupported",
                _ => "policy-lint",
            },
            Self::Canonical(_) => "policy-canonicalization",
        }
    }

    pub fn issues(&self) -> &[LintIssue] {
        match self {
            Self::Lint { issues } => issues,
            _ => &[],
        }
    }
}

pub fn compile(sources: &[PolicySource<'_>]) -> Result<EffectivePolicy, PolicyError> {
    compile_at(sources, Timestamp::now())
}

pub fn compile_at(
    sources: &[PolicySource<'_>],
    now: Timestamp,
) -> Result<EffectivePolicy, PolicyError> {
    let (contracts, declarations, mut source_records) = parse_sources(sources)?;
    let (mut contracts, mut issues) = expand(contracts);
    issues.extend(lint_effective(&contracts, &declarations, now));
    sort_issues(&mut issues);
    // Spec 7.6: an expired migration still present is reported and stops applying;
    // it does not abort compilation. Every other structural issue stays fatal.
    let (reports, issues): (Vec<_>, Vec<_>) = issues
        .into_iter()
        .partition(|issue| issue.code == EXPIRED_CONTRACT);
    if !issues.is_empty() {
        return Err(PolicyError::Lint { issues });
    }

    contracts.sort_by(|left, right| left.id.cmp(&right.id));
    let mut declarations = declarations;
    declarations.sort();
    source_records.sort_by(|left, right| left.path.cmp(&right.path));
    // The digest covers the declared contracts, expired ones included, so it does not
    // change on a clock tick with no policy edit (rulings bind to it, spec section 11).
    let policy_digest = semantic_digest(&contracts, &declarations)?;
    let expired: BTreeSet<&str> = reports
        .iter()
        .flat_map(|report| report.contracts.iter().map(String::as_str))
        .collect();
    contracts.retain(|contract| !expired.contains(contract.id.as_str()));
    Ok(EffectivePolicy {
        schema_version: EFFECTIVE_POLICY_SCHEMA.into(),
        policy_digest,
        contracts,
        declarations,
        reports,
        sources: source_records,
    })
}

pub fn lint_at(
    sources: &[PolicySource<'_>],
    now: Timestamp,
) -> Result<Vec<LintIssue>, PolicyError> {
    let (contracts, declarations, _) = parse_sources(sources)?;
    let (contracts, mut issues) = expand(contracts);
    issues.extend(lint_effective(&contracts, &declarations, now));
    sort_issues(&mut issues);
    Ok(issues)
}

fn parse_sources(
    sources: &[PolicySource<'_>],
) -> Result<(Vec<PolicyContract>, Declarations, Vec<PolicySourceRecord>), PolicyError> {
    let mut contracts = Vec::new();
    let mut declarations = Declarations::default();
    let mut records = Vec::new();
    for source in sources {
        let document: PolicyDocument =
            serde_saphyr::from_str(source.yaml).map_err(|error| PolicyError::Parse {
                path: source.path.into(),
                reason: error.to_string(),
            })?;
        if document.schema_version != POLICY_SCHEMA {
            return Err(PolicyError::SchemaVersion {
                path: source.path.into(),
                actual: document.schema_version,
            });
        }
        contracts.extend(document.contracts);
        declarations.extend(document.declarations);
        records.push(PolicySourceRecord {
            path: source.path.into(),
            digest: digest_bytes(source.yaml.as_bytes()),
        });
    }
    Ok((contracts, declarations, records))
}

fn expand(contracts: Vec<PolicyContract>) -> (Vec<EffectiveContract>, Vec<LintIssue>) {
    let mut effective = Vec::with_capacity(contracts.len());
    let mut issues = Vec::new();
    for contract in contracts {
        let kind = contract.body.kind();
        let fixed_claim = default_claim(kind);
        let claim = contract.claim.unwrap_or(fixed_claim);
        if claim != fixed_claim {
            issues.push(issue(
                "claim-kind",
                [&contract.id],
                format!(
                    "contract kind `{}` has fixed claim `{}`; the policy supplied `{}`",
                    kind_name(kind),
                    claim_name(fixed_claim),
                    claim_name(claim)
                ),
            ));
        }
        let expected = claim_capabilities(kind, claim).unwrap_or_default();
        let required = contract
            .requires_capabilities
            .clone()
            .unwrap_or_else(|| expected.clone());
        if required != expected {
            issues.push(issue(
                "claim-capabilities",
                [&contract.id],
                format!(
                    "claim `{}` on `{}` requires exactly {}; the policy supplied {}",
                    claim_name(claim),
                    kind_name(kind),
                    capability_names(&expected),
                    capability_names(&required)
                ),
            ));
        }
        let on_violation = contract
            .on_violation
            .unwrap_or_else(|| default_consequence(contract.class));
        if contract.class == ContractClass::Invariant && on_violation == Consequence::Note {
            issues.push(issue(
                "invariant-advice",
                [&contract.id],
                "an invariant cannot use `on_violation: note`".into(),
            ));
        }
        effective.push(EffectiveContract {
            id: contract.id,
            intent: contract.intent,
            owner: contract.owner,
            authority: contract.authority,
            class: contract.class,
            expires: contract.expires,
            claim,
            requires_capabilities: required,
            enforcement: contract.enforcement,
            limits: contract.limits,
            on_violation,
            supersedes: contract.supersedes,
            overrides: contract.overrides,
            body: contract.body,
        });
    }
    (effective, issues)
}

fn lint_effective(
    contracts: &[EffectiveContract],
    declarations: &Declarations,
    now: Timestamp,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    lint_unique_ids(contracts, declarations, &mut issues);
    let ids: BTreeSet<&str> = contracts
        .iter()
        .map(|contract| contract.id.as_str())
        .collect();
    for contract in contracts {
        lint_authority(contract, &ids, &mut issues);
        lint_migration(contract, now, &mut issues);
        lint_enforcement(contract, &mut issues);
    }
    lint_module_conflicts(contracts, &mut issues);
    lint_dependency_conflicts(contracts, &mut issues);
    lint_state_conflicts(contracts, &mut issues);
    lint_interface_exports(contracts, &mut issues);
    issues
}

fn lint_unique_ids(
    contracts: &[EffectiveContract],
    declarations: &Declarations,
    issues: &mut Vec<LintIssue>,
) {
    let mut seen = BTreeSet::new();
    for id in contracts.iter().map(|contract| contract.id.as_str()).chain(
        declarations
            .stores
            .iter()
            .map(|item| item.id.as_str())
            .chain(declarations.registries.iter().map(|item| item.id.as_str()))
            .chain(
                declarations
                    .external_consumers
                    .iter()
                    .map(|item| item.id.as_str()),
            )
            .chain(declarations.loaders.iter().map(|item| item.id.as_str())),
    ) {
        if !seen.insert(id) {
            issues.push(issue(
                "duplicate-id",
                [id],
                format!("id `{id}` is declared more than once"),
            ));
        }
    }
}

fn lint_authority(contract: &EffectiveContract, ids: &BTreeSet<&str>, issues: &mut Vec<LintIssue>) {
    if contract.authority != "draft" && !contract.authority.starts_with("ruling:") {
        issues.push(issue(
            "invalid-authority",
            [&contract.id],
            "authority must be `draft` or start with `ruling:`".into(),
        ));
    }
    if !contract.overrides.is_empty() && !contract.authority.starts_with("ruling:") {
        issues.push(issue(
            "override-without-authority",
            [&contract.id],
            "a contract with `overrides` requires `authority: ruling:<id>`".into(),
        ));
    }
    for target in &contract.overrides {
        if !ids.contains(target.as_str()) {
            issues.push(issue(
                "unknown-override",
                [&contract.id],
                format!("override target `{target}` does not exist"),
            ));
        }
    }
}

fn lint_migration(contract: &EffectiveContract, now: Timestamp, issues: &mut Vec<LintIssue>) {
    if contract.class != ContractClass::Migration {
        return;
    }
    let Some(expires) = contract.expires.as_deref() else {
        issues.push(issue(
            "migration-missing-expiry",
            [&contract.id],
            "a migration contract requires `expires`".into(),
        ));
        return;
    };
    match expires.parse::<Timestamp>() {
        Ok(expiry) if expiry <= now => issues.push(issue(
            EXPIRED_CONTRACT,
            [&contract.id],
            format!("migration expired at {expiry}"),
        )),
        Ok(_) => {}
        Err(error) => issues.push(issue(
            "invalid-expiry",
            [&contract.id],
            format!("`expires` is not an RFC 3339 timestamp: {error}"),
        )),
    }
}

fn lint_enforcement(contract: &EffectiveContract, issues: &mut Vec<LintIssue>) {
    let supported = enforcement_capabilities(contract.enforcement);
    let missing: BTreeSet<_> = contract
        .requires_capabilities
        .difference(&supported)
        .copied()
        .collect();
    if !missing.is_empty() {
        issues.push(issue(
            "enforcement-unsupported",
            [&contract.id],
            format!(
                "enforcement `{}` cannot supply {}",
                enforcement_name(contract.enforcement),
                capability_names(&missing)
            ),
        ));
    }
}

fn lint_module_conflicts(contracts: &[EffectiveContract], issues: &mut Vec<LintIssue>) {
    for (index, left) in contracts.iter().enumerate() {
        let ContractBody::Module(left_module) = &left.body else {
            continue;
        };
        for right in &contracts[index + 1..] {
            let ContractBody::Module(right_module) = &right.body else {
                continue;
            };
            if left_module.files.iter().any(|left_pattern| {
                right_module
                    .files
                    .iter()
                    .any(|right_pattern| file_selectors_overlap(left_pattern, right_pattern))
            }) {
                issues.push(issue(
                    "module-files-conflict",
                    [&left.id, &right.id],
                    "module file selectors overlap".into(),
                ));
            }
        }
    }
}

fn lint_dependency_conflicts(contracts: &[EffectiveContract], issues: &mut Vec<LintIssue>) {
    for (index, left) in contracts.iter().enumerate() {
        let ContractBody::Dependency(left_dependency) = &left.body else {
            continue;
        };
        for right in &contracts[index..] {
            let ContractBody::Dependency(right_dependency) = &right.body else {
                continue;
            };
            if !selector_sets_overlap(
                &left_dependency.from.modules,
                &right_dependency.from.modules,
                module_selectors_overlap,
            ) || overrides(left, right)
            {
                continue;
            }
            if targets_overlap(&left_dependency.allow, &right_dependency.deny)
                || targets_overlap(&right_dependency.allow, &left_dependency.deny)
            {
                issues.push(issue(
                    "dependency-conflict",
                    [&left.id, &right.id],
                    "the same dependency edge is both allowed and denied".into(),
                ));
            }
        }
    }
}

fn lint_state_conflicts(contracts: &[EffectiveContract], issues: &mut Vec<LintIssue>) {
    for (index, left) in contracts.iter().enumerate() {
        let ContractBody::State(left_state) = &left.body else {
            continue;
        };
        for right in &contracts[index + 1..] {
            let ContractBody::State(right_state) = &right.body else {
                continue;
            };
            if left_state.store == right_state.store
                && left_state.writers.modules != right_state.writers.modules
                && !overrides(left, right)
            {
                issues.push(issue(
                    "state-writers-conflict",
                    [&left.id, &right.id],
                    format!("store `{}` has different writer sets", left_state.store),
                ));
            }
        }
    }
}

fn lint_interface_exports(contracts: &[EffectiveContract], issues: &mut Vec<LintIssue>) {
    let modules: BTreeMap<_, _> = contracts
        .iter()
        .filter_map(|contract| match &contract.body {
            ContractBody::Module(module) => Some((contract.id.as_str(), module)),
            _ => None,
        })
        .collect();
    for contract in contracts {
        let ContractBody::Interface(interface) = &contract.body else {
            continue;
        };
        let Some(module) = modules.get(interface.module.as_str()) else {
            issues.push(issue(
                "interface-module-missing",
                [&contract.id],
                format!("module `{}` is not declared", interface.module),
            ));
            continue;
        };
        let missing: Vec<_> = interface
            .exports
            .difference(&module.interface.exports)
            .cloned()
            .collect();
        if !missing.is_empty() {
            issues.push(issue(
                "interface-export-missing",
                [&contract.id],
                format!(
                    "module `{}` does not export {}",
                    interface.module,
                    missing.join(", ")
                ),
            ));
        }
    }
}

fn semantic_digest(
    contracts: &[EffectiveContract],
    declarations: &Declarations,
) -> Result<String, PolicyError> {
    #[derive(Serialize)]
    struct SemanticPolicy<'a> {
        schema_version: &'static str,
        contracts: &'a [EffectiveContract],
        declarations: &'a Declarations,
    }
    let mut value = serde_json::to_value(SemanticPolicy {
        schema_version: EFFECTIVE_POLICY_SCHEMA,
        contracts,
        declarations,
    })?;
    if let Some(contracts) = value
        .get_mut("contracts")
        .and_then(serde_json::Value::as_array_mut)
    {
        for contract in contracts {
            if let Some(object) = contract.as_object_mut() {
                object.remove("authority");
            }
        }
    }
    let bytes = serde_json_canonicalizer::to_vec(&value)?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

fn default_claim(kind: ContractKind) -> Claim {
    match kind {
        ContractKind::Module => Claim::DeclaredOwnership,
        ContractKind::Dependency => Claim::ResolvedDependencyBoundary,
        ContractKind::Interface | ContractKind::Effect | ContractKind::Data => {
            Claim::ObservedConsumerBoundary
        }
        ContractKind::State => Claim::RecognizedWriteBoundary,
        ContractKind::Capability | ContractKind::Pattern => Claim::RequiredStructure,
        ContractKind::Evidence => Claim::TestedBehavior,
    }
}

/// The closed claim vocabulary's fixed capability requirements for one contract kind.
pub fn claim_capabilities(kind: ContractKind, claim: Claim) -> Option<BTreeSet<Capability>> {
    use Capability::{
        AuthenticatedTestCaseResults, BindingReferences, ModuleResolution, RecognizedCallSites,
        StructuralMatch,
    };
    let capabilities: &[Capability] = match (kind, claim) {
        (ContractKind::Module, Claim::DeclaredOwnership) => &[],
        (ContractKind::Dependency, Claim::ResolvedDependencyBoundary) => &[ModuleResolution],
        (ContractKind::Interface, Claim::ObservedConsumerBoundary)
        | (ContractKind::Data, Claim::ObservedConsumerBoundary) => &[BindingReferences],
        (ContractKind::Effect, Claim::ObservedConsumerBoundary) => {
            &[BindingReferences, RecognizedCallSites, StructuralMatch]
        }
        (ContractKind::State, Claim::RecognizedWriteBoundary) => &[StructuralMatch],
        (ContractKind::Capability, Claim::RequiredStructure) => {
            &[BindingReferences, StructuralMatch]
        }
        (ContractKind::Pattern, Claim::RequiredStructure) => &[StructuralMatch],
        (ContractKind::Evidence, Claim::TestedBehavior) => &[AuthenticatedTestCaseResults],
        _ => return None,
    };
    Some(capabilities.iter().copied().collect())
}

fn enforcement_capabilities(enforcement: Enforcement) -> BTreeSet<Capability> {
    use Capability::{
        AuthenticatedTestCaseResults, BindingReferences, ModuleResolution, RecognizedCallSites,
        StructuralMatch,
    };
    let capabilities: &[Capability] = match enforcement {
        Enforcement::Static => &[ModuleResolution, BindingReferences],
        Enforcement::Pattern => &[RecognizedCallSites, StructuralMatch],
        Enforcement::Evidence => &[AuthenticatedTestCaseResults],
        Enforcement::Mixed => &[
            ModuleResolution,
            BindingReferences,
            RecognizedCallSites,
            StructuralMatch,
            AuthenticatedTestCaseResults,
        ],
    };
    capabilities.iter().copied().collect()
}

fn default_consequence(class: ContractClass) -> Consequence {
    match class {
        ContractClass::Invariant | ContractClass::Migration => Consequence::Fail,
        ContractClass::Preference => Consequence::Note,
    }
}

fn overrides(left: &EffectiveContract, right: &EffectiveContract) -> bool {
    left.overrides.contains(&right.id) || right.overrides.contains(&left.id)
}

fn targets_overlap(left: &DependencyTargets, right: &DependencyTargets) -> bool {
    selector_sets_overlap(&left.modules, &right.modules, module_selectors_overlap)
        || selector_sets_overlap(&left.packages, &right.packages, package_selectors_overlap)
}

fn selector_sets_overlap(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    overlaps: fn(&str, &str) -> bool,
) -> bool {
    left.iter().any(|left_item| {
        right
            .iter()
            .any(|right_item| overlaps(left_item, right_item))
    })
}

fn segment_prefix(value: &str, prefix: &str, separator: char) -> bool {
    value == prefix
        || value
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(separator))
}

fn file_selectors_overlap(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    match (left.strip_suffix("/**"), right.strip_suffix("/**")) {
        (Some(left_prefix), Some(right_prefix)) => {
            segment_prefix(left_prefix, right_prefix, '/')
                || segment_prefix(right_prefix, left_prefix, '/')
        }
        (Some(prefix), None) => segment_prefix(right, prefix, '/'),
        (None, Some(prefix)) => segment_prefix(left, prefix, '/'),
        (None, None) => false,
    }
}

fn module_selectors_overlap(left: &str, right: &str) -> bool {
    let matches = |pattern: &str, value: &str| {
        pattern.strip_suffix('*').is_some_and(|prefix| {
            prefix.is_empty() || segment_prefix(value, prefix.trim_end_matches('.'), '.')
        })
    };
    left == right || matches(left, right) || matches(right, left)
}

fn package_selectors_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_suffix('*')
            .is_some_and(|prefix| right.starts_with(prefix))
        || right
            .strip_suffix('*')
            .is_some_and(|prefix| left.starts_with(prefix))
}

fn issue<I, S>(code: &str, contracts: I, reason: String) -> LintIssue
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    LintIssue {
        code: code.into(),
        contracts: contracts
            .into_iter()
            .map(|contract| contract.as_ref().to_owned())
            .collect(),
        reason,
    }
}

fn sort_issues(issues: &mut [LintIssue]) {
    issues.sort_by(|left, right| {
        (&left.code, &left.contracts, &left.reason).cmp(&(
            &right.code,
            &right.contracts,
            &right.reason,
        ))
    });
}

fn format_issues(issues: &[LintIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}", issue.code, issue.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn claim_name(claim: Claim) -> &'static str {
    match claim {
        Claim::DeclaredOwnership => "declared-ownership",
        Claim::ResolvedDependencyBoundary => "resolved-dependency-boundary",
        Claim::ObservedConsumerBoundary => "observed-consumer-boundary",
        Claim::RecognizedWriteBoundary => "recognized-write-boundary",
        Claim::RequiredStructure => "required-structure",
        Claim::TestedBehavior => "tested-behavior",
    }
}

fn kind_name(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Module => "module",
        ContractKind::Dependency => "dependency",
        ContractKind::Interface => "interface",
        ContractKind::Effect => "effect",
        ContractKind::State => "state",
        ContractKind::Capability => "capability",
        ContractKind::Pattern => "pattern",
        ContractKind::Evidence => "evidence",
        ContractKind::Data => "data",
    }
}

fn enforcement_name(enforcement: Enforcement) -> &'static str {
    match enforcement {
        Enforcement::Static => "static",
        Enforcement::Pattern => "pattern",
        Enforcement::Evidence => "evidence",
        Enforcement::Mixed => "mixed",
    }
}

fn capability_names(capabilities: &BTreeSet<Capability>) -> String {
    capabilities
        .iter()
        .map(|capability| match capability {
            Capability::ModuleResolution => "module-resolution",
            Capability::BindingReferences => "binding-references",
            Capability::RecognizedCallSites => "recognized-call-sites",
            Capability::StructuralMatch => "structural-match",
            Capability::AuthenticatedTestCaseResults => "authenticated-test-case-results",
            Capability::CompilerReferences => "compiler-references",
            Capability::ControlFlow => "control-flow",
            Capability::TaintFlow => "taint-flow",
        })
        .collect::<Vec<_>>()
        .join(", ")
}
