use std::{fs, path::PathBuf};

use jiff::Timestamp;

use super::{
    Capability, Claim, Consequence, ContractKind, PolicyError, PolicySource, claim_capabilities,
    compile_at, lint_at,
};

const MODULE: &str = r#"
schema_version: warrant.policy/1
contracts:
  - id: core.actions
    kind: module
    intent: The dispatcher owns actions.
    owner: trey
    authority: draft
    files: ["src/actions/**"]
    interface:
      entry: src/actions/index.ts
      exports: [run]
    owns:
      responsibilities: [dispatch]
      stores: []
    dynamic_loads: forbid
    enforcement: static
    limits: Ownership is declared; edges are observed.
"#;

const DEPENDENCY: &str = r#"
schema_version: warrant.policy/1
contracts:
- limits: Binding-level dependencies only.
  enforcement: static
  deny:
    packages: [react]
  from:
    modules: [core.actions]
  authority: draft
  owner: trey
  intent: Core does not import UI packages.
  kind: dependency
  id: layering.core
"#;

#[test]
fn policy_digest_ignores_file_placement_order_and_yaml_formatting() {
    let now: Timestamp = "2026-09-24T08:00:00Z".parse().expect("fixed timestamp");
    let split = compile_at(
        &[
            PolicySource::new("warrant/policy/modules.yaml", MODULE),
            PolicySource::new("warrant/policy/dependencies.yaml", DEPENDENCY),
        ],
        now,
    )
    .expect("split policy compiles");
    let combined_yaml = format!(
        "schema_version: warrant.policy/1\ncontracts:\n{}\n{}",
        contract_body(DEPENDENCY),
        contract_body(MODULE)
    );
    let combined = compile_at(
        &[PolicySource::new("moved/reformatted.yaml", &combined_yaml)],
        now,
    )
    .expect("combined policy compiles");

    assert_eq!(split.policy_digest, combined.policy_digest);
    assert_eq!(split.contracts, combined.contracts);
}

fn contract_body(document: &str) -> String {
    let value: serde_json::Value = serde_saphyr::from_str(document).expect("fixture policy");
    let contract = &value["contracts"][0];
    let yaml = serde_saphyr::to_string(contract).expect("serialize contract");
    yaml.lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("  - {line}\n")
            } else {
                format!("    {line}\n")
            }
        })
        .collect()
}

#[test]
fn all_nine_contract_kinds_compile_with_documented_defaults() {
    let effective = compile_one(ALL_KINDS).expect("all contract kinds compile");

    assert_eq!(effective.contracts.len(), 9);
    assert_eq!(
        effective
            .contracts
            .iter()
            .find(|contract| contract.id == "style.named")
            .expect("pattern preference")
            .on_violation,
        Consequence::Note
    );
    assert!(
        effective
            .contracts
            .iter()
            .filter(|contract| contract.id != "style.named")
            .all(|contract| contract.on_violation == Consequence::Fail)
    );
    assert_eq!(
        claim_capabilities(ContractKind::Effect, Claim::ObservedConsumerBoundary)
            .expect("effect claim mapping"),
        [
            Capability::BindingReferences,
            Capability::RecognizedCallSites,
            Capability::StructuralMatch,
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn authority_pointer_changes_do_not_change_policy_digest() {
    let draft = compile_one(ALL_KINDS).expect("draft policy");
    let ruled_yaml = ALL_KINDS.replace("authority: draft", "authority: ruling:bootstrap");
    let ruled = compile_one(&ruled_yaml).expect("ruled policy");

    assert_eq!(draft.policy_digest, ruled.policy_digest);
    assert_ne!(draft.contracts, ruled.contracts);
}

#[test]
fn claim_capabilities_cannot_be_lowered_in_policy() {
    let yaml = ALL_KINDS.replace(
        "requires_capabilities: [binding-references, recognized-call-sites, structural-match]",
        "requires_capabilities: [binding-references]",
    );
    let error = compile_one(&yaml).expect_err("lowered capabilities must fail");

    assert_issue(&error, "claim-capabilities");
}

#[test]
fn fixed_claim_cannot_be_replaced_with_a_weaker_vocabulary_item() {
    let yaml = ALL_KINDS.replace(
        "claim: observed-consumer-boundary\n    requires_capabilities: [binding-references, recognized-call-sites, structural-match]",
        "claim: declared-ownership\n    requires_capabilities: []",
    );
    let error = compile_one(&yaml).expect_err("changed fixed claim must fail");

    assert_issue(&error, "claim-kind");
}

#[test]
fn unsupported_enforcement_names_the_missing_capability() {
    let yaml = ALL_KINDS.replace(
        "permitted_sinks: { modules: [core.actions] }\n    enforcement: static",
        "permitted_sinks: { modules: [core.actions] }\n    enforcement: pattern",
    );
    let error = compile_one(&yaml).expect_err("pattern cannot supply binding references");

    assert_issue(&error, "enforcement-unsupported");
    assert!(error.to_string().contains("binding-references"));
}

#[test]
fn structural_lint_rejects_duplicate_ids_draft_overrides_and_expired_migrations() {
    let yaml = r#"
schema_version: warrant.policy/1
contracts:
  - &base
    id: duplicate
    kind: pattern
    intent: A temporary structural rule.
    owner: trey
    authority: draft
    class: migration
    expires: 2026-09-23T00:00:00Z
    scope: { modules: [core.actions] }
    rule: { pattern: "x()" }
    enforcement: pattern
    limits: Syntactic.
    overrides: [other]
  - <<: *base
    id: duplicate
  - <<: *base
    id: other
    class: invariant
    expires: null
    overrides: []
"#;
    let issues =
        lint_at(&[PolicySource::new("policy.yaml", yaml)], fixed_now()).expect("policy parses");
    let codes: Vec<_> = issues.iter().map(|issue| issue.code.as_str()).collect();

    assert!(codes.contains(&"duplicate-id"), "{codes:?}");
    assert!(codes.contains(&"override-without-authority"), "{codes:?}");
    assert!(codes.contains(&"expired-contract"), "{codes:?}");
}

#[test]
fn expired_migration_is_reported_and_stops_applying_under_a_stable_digest() {
    let yaml = r#"
schema_version: warrant.policy/1
contracts:
  - id: core.actions
    kind: module
    intent: The dispatcher owns actions.
    owner: trey
    authority: draft
    files: ["src/actions/**"]
    interface: { entry: src/actions/index.ts, exports: [run] }
    enforcement: static
    limits: Ownership is declared.
  - id: temporary.rule
    kind: pattern
    intent: A temporary structural rule.
    owner: trey
    authority: draft
    class: migration
    expires: 2026-09-24T00:00:00Z
    scope: { modules: [core.actions] }
    rule: { pattern: "x()" }
    enforcement: pattern
    limits: Syntactic.
"#;
    let sources = [PolicySource::new("policy.yaml", yaml)];
    let before = compile_at(&sources, "2026-09-23T23:59:59Z".parse().expect("timestamp"))
        .expect("an unexpired migration compiles");
    let after = compile_at(&sources, fixed_now()).expect("an expired migration still compiles");

    assert_eq!(
        before.policy_digest, after.policy_digest,
        "digest is clock-independent"
    );
    assert!(before.reports.is_empty(), "{:?}", before.reports);
    assert_eq!(before.contracts.len(), 2);
    assert_eq!(
        after
            .contracts
            .iter()
            .map(|contract| contract.id.as_str())
            .collect::<Vec<_>>(),
        ["core.actions"],
        "the expired migration stops applying"
    );
    assert_eq!(after.reports.len(), 1, "{:?}", after.reports);
    assert_eq!(after.reports[0].code, "expired-contract");
    assert_eq!(after.reports[0].contracts, ["temporary.rule"]);
}

#[test]
fn migration_without_expiry_still_fails_compilation() {
    let yaml = r#"
schema_version: warrant.policy/1
contracts:
  - id: temporary.rule
    kind: pattern
    intent: A temporary structural rule.
    owner: trey
    authority: draft
    class: migration
    scope: { modules: [core.actions] }
    rule: { pattern: "x()" }
    enforcement: pattern
    limits: Syntactic.
"#;
    let error = compile_one(yaml).expect_err("a migration needs an expiry");
    assert_issue(&error, "migration-missing-expiry");
    assert_eq!(error.code(), "policy-lint");
}

#[test]
fn structural_conflict_classes_compile_without_a_snapshot() {
    let error = compile_one(CONFLICTS).expect_err("structural conflicts must fail");
    let codes: Vec<_> = error
        .issues()
        .iter()
        .map(|issue| issue.code.as_str())
        .collect();

    for code in [
        "module-files-conflict",
        "dependency-conflict",
        "state-writers-conflict",
        "interface-export-missing",
    ] {
        assert!(codes.contains(&code), "missing {code}: {codes:?}");
    }
}

#[test]
fn policy_conformance_fixtures_cover_digest_conflict_and_unsupported_enforcement() {
    let digest_case = fixture_root().join("digest-stability/warrant");
    let original_files = fixture_files(&digest_case.join("original"));
    let original_sources: Vec<_> = original_files
        .iter()
        .map(|(path, yaml)| PolicySource::new(path, yaml))
        .collect();
    let original = compile_at(&original_sources, fixed_now()).expect("original digest fixture");
    let reformatted_files = fixture_files(&digest_case.join("reformatted"));
    let reformatted_sources: Vec<_> = reformatted_files
        .iter()
        .map(|(path, yaml)| PolicySource::new(path, yaml))
        .collect();
    let reformatted =
        compile_at(&reformatted_sources, fixed_now()).expect("reformatted digest fixture");
    assert_eq!(original.policy_digest, reformatted.policy_digest);

    let conflict = fixture_root().join("structural-conflict/warrant/policy.yaml");
    let conflict_yaml = fs::read_to_string(&conflict).expect("conflict fixture");
    assert_issue(
        &compile_one(&conflict_yaml).expect_err("conflict fixture must fail"),
        "module-files-conflict",
    );

    let unsupported = fixture_root().join("enforcement-unsupported/warrant/policy.yaml");
    let unsupported_yaml = fs::read_to_string(&unsupported).expect("unsupported fixture");
    assert_issue(
        &compile_one(&unsupported_yaml).expect_err("unsupported fixture must fail"),
        "enforcement-unsupported",
    );
}

fn compile_one(yaml: &str) -> Result<super::EffectivePolicy, PolicyError> {
    compile_at(&[PolicySource::new("policy.yaml", yaml)], fixed_now())
}

fn fixed_now() -> Timestamp {
    "2026-09-24T08:00:00Z".parse().expect("fixed timestamp")
}

fn assert_issue(error: &PolicyError, code: &str) {
    assert!(
        error.issues().iter().any(|issue| issue.code == code),
        "missing {code}: {error}"
    );
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/policy")
}

fn fixture_files(directory: &std::path::Path) -> Vec<(String, String)> {
    let mut paths: Vec<_> = fs::read_dir(directory)
        .expect("fixture directory")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let display = path.to_string_lossy().into_owned();
            let yaml = fs::read_to_string(path).expect("fixture YAML");
            (display, yaml)
        })
        .collect()
}

const ALL_KINDS: &str = r#"
schema_version: warrant.policy/1
declarations:
  stores:
    - { id: approvals, kind: table, defined_in: src/schema.ts }
  registries:
    - id: undo-registry
      intent: Undo registrations.
      file: src/undo.ts
      rule: { pattern: "registerUndo($X)" }
      captures: { symbol: X }
      entrypoint_kind: registry
contracts:
  - id: core.actions
    kind: module
    intent: Own action dispatch.
    owner: trey
    authority: draft
    files: ["src/actions/**"]
    interface: { entry: src/actions/index.ts, exports: [run] }
    owns: { responsibilities: [dispatch], stores: [approvals] }
    enforcement: static
    limits: Ownership is declared.
  - id: dep.core
    kind: dependency
    intent: Keep core independent.
    owner: trey
    authority: draft
    from: { modules: [core.actions] }
    deny: { packages: [react] }
    enforcement: static
    limits: Resolved edges only.
  - id: iface.actions
    kind: interface
    intent: Use the public entry.
    owner: trey
    authority: draft
    module: core.actions
    only_via_entry: true
    consumers: { modules: [web] }
    exports: [run]
    enforcement: static
    limits: Binding consumers only.
  - id: effect.send
    kind: effect
    intent: Centralize sends.
    owner: trey
    authority: draft
    claim: observed-consumer-boundary
    requires_capabilities: [binding-references, recognized-call-sites, structural-match]
    interface: { symbols: ["core.actions::run"] }
    permitted_callers: { modules: [worker] }
    required_context: { pattern: { rule: { pattern: "run($CTX)" } } }
    enforcement: mixed
    limits: Recognized call sites only.
  - id: state.approvals
    kind: state
    intent: Restrict writes.
    owner: trey
    authority: draft
    store: approvals
    writers: { modules: [core.actions] }
    write_sites: { pattern: { rule: { pattern: "insert(approvals)" } } }
    enforcement: pattern
    limits: Recognized writes only.
  - id: capability.undo
    kind: capability
    intent: Link undo structure.
    owner: trey
    authority: draft
    instances: { registry: undo-registry }
    requires: [{ link: entrypoint, basis: registry }]
    enforcement: mixed
    limits: Structural links only.
  - id: style.named
    kind: pattern
    class: preference
    intent: Prefer named exports.
    owner: trey
    authority: draft
    scope: { modules: [core.actions] }
    rule: { pattern: "export default $X" }
    forbid: true
    enforcement: pattern
    limits: Syntactic.
  - id: evidence.worker
    kind: evidence
    intent: Require the worker suite.
    owner: trey
    authority: draft
    requires: { kind: test-receipt, tag: worker, evidence_kind: integration }
    enforcement: evidence
    limits: The receipt proves only its named cases.
  - id: data.private
    kind: data
    intent: Restrict private data imports.
    owner: trey
    authority: draft
    sources: { symbols: ["mail::fetch"] }
    permitted_sinks: { modules: [core.actions] }
    enforcement: static
    limits: Binding-level, not taint flow.
"#;

const CONFLICTS: &str = r#"
schema_version: warrant.policy/1
declarations:
  stores:
    - { id: approvals, kind: table, defined_in: src/schema.ts }
contracts:
  - id: module.one
    kind: module
    intent: First owner.
    owner: trey
    authority: draft
    files: ["src/shared/**"]
    interface: { entry: src/shared/index.ts, exports: [run] }
    enforcement: static
    limits: Declared ownership.
  - id: module.two
    kind: module
    intent: Conflicting owner.
    owner: trey
    authority: draft
    files: ["src/shared/**"]
    interface: { entry: src/shared/other.ts, exports: [] }
    enforcement: static
    limits: Declared ownership.
  - id: dep.allow
    kind: dependency
    intent: Allow an edge.
    owner: trey
    authority: draft
    from: { modules: [module.one] }
    allow: { packages: [react] }
    enforcement: static
    limits: Resolved edges.
  - id: dep.deny
    kind: dependency
    intent: Deny the same edge.
    owner: trey
    authority: draft
    from: { modules: [module.one] }
    deny: { packages: [react] }
    enforcement: static
    limits: Resolved edges.
  - id: state.one
    kind: state
    intent: First writer set.
    owner: trey
    authority: draft
    store: approvals
    writers: { modules: [module.one] }
    write_sites: { pattern: { rule: { pattern: "insert($X)" } } }
    enforcement: pattern
    limits: Recognized writes.
  - id: state.two
    kind: state
    intent: Second writer set.
    owner: trey
    authority: draft
    store: approvals
    writers: { modules: [module.two] }
    write_sites: { pattern: { rule: { pattern: "insert($X)" } } }
    enforcement: pattern
    limits: Recognized writes.
  - id: iface.missing
    kind: interface
    intent: Name an absent export.
    owner: trey
    authority: draft
    module: module.one
    exports: [missing]
    enforcement: static
    limits: Declared exports.
"#;
