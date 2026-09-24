use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use serde::Deserialize;
use serde_json::{Value, json};
use warrant_core::{
    manifest::WarrantManifest,
    nouns::{GeneratedDrift, InventoryClass, InventoryDocument, SnapshotKind},
    policy::EffectivePolicy,
};
use warrant_inventory::{BuildConfig, ClassRule, ModuleSelector};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Driver {
    Cli,
    InventoryApi,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    #[serde(default)]
    force_add: Vec<String>,
    #[serde(default)]
    stage_files: BTreeMap<String, String>,
    #[serde(default)]
    worktree_files: BTreeMap<String, String>,
    /// Paths that differ only by case from a checked-in fixture path. The runner writes
    /// each one and stages its blob with `update-index --cacheinfo`, so the pair is never
    /// checked in: a clone on a case-insensitive filesystem would drop one of them, and
    /// Warrant's own snapshot would report the collision.
    #[serde(default)]
    case_variant_files: BTreeMap<String, String>,
    #[serde(default)]
    symlinks: BTreeMap<String, String>,
    #[serde(default)]
    local_submodule: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleInput {
    id: String,
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClassRuleInput {
    id: String,
    class: InventoryClass,
    files: Vec<String>,
    #[serde(default)]
    replaces: Option<InventoryClass>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryExpect {
    #[serde(default)]
    classes: BTreeMap<String, InventoryClass>,
    #[serde(default)]
    modules: BTreeMap<String, Option<String>>,
    #[serde(default)]
    reasons: BTreeMap<String, String>,
    #[serde(default)]
    by_class: BTreeMap<String, u64>,
    #[serde(default)]
    unowned_source: Option<Vec<String>>,
    #[serde(default)]
    unknown: Option<Vec<String>>,
    #[serde(default)]
    unread: Option<BTreeMap<String, String>>,
    #[serde(default)]
    generated_absent: Option<Vec<String>>,
    /// Declaration to producer for each `generated_absent` row named here.
    #[serde(default)]
    generated_absent_producers: BTreeMap<String, String>,
    #[serde(default)]
    ignored_files: Option<u64>,
    #[serde(default)]
    submodules: Option<Vec<String>>,
    /// The `--verify-generated` drift rows, compared whole; a document whose producers
    /// were not re-run (null) never matches.
    #[serde(default)]
    generated_drift: Option<Vec<GeneratedDrift>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorExpect {
    code: String,
    #[serde(default)]
    reason_contains: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    driver: Driver,
    #[serde(default)]
    args: Vec<String>,
    exit_code: i32,
    #[serde(default)]
    setup: Setup,
    #[serde(default)]
    modules: Vec<ModuleInput>,
    #[serde(default)]
    class_rules: Vec<ClassRuleInput>,
    #[serde(default)]
    document: Option<Value>,
    #[serde(default)]
    inventory: Option<InventoryExpect>,
    #[serde(default)]
    error: Option<ErrorExpect>,
    #[serde(default)]
    tree_matches_index: Option<bool>,
    /// What `git ls-files --others --ignored --exclude-standard` prints for the case.
    #[serde(default)]
    git_ignored: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyExpectation {
    exit_code: i32,
    #[serde(default)]
    equal_policy_digest: Option<bool>,
    #[serde(default)]
    source_layout_differs: Option<bool>,
    #[serde(default)]
    error: Option<ErrorExpect>,
    /// Non-fatal reports on the effective policy, compared order-insensitively.
    #[serde(default)]
    reports: Option<Vec<ReportExpect>>,
    /// The exact set of contract ids in the effective policy.
    #[serde(default)]
    contract_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportExpect {
    code: String,
    contracts: Vec<String>,
}

#[derive(Debug)]
struct Observation {
    exit_code: i32,
    document: Option<Value>,
    stderr: String,
    index_tree: Option<String>,
    git_ignored: Option<Vec<String>>,
}

#[test]
fn inventory_conformance() {
    run_area("inventory");
}

#[test]
fn snapshot_conformance() {
    run_area("snapshot");
}

mod policy_conformance {
    macro_rules! cases {
        ($($test:ident => $case:literal),+ $(,)?) => {
            const CASES: &[&str] = &[$($case),+];
            $(#[test]
            fn $test() {
                super::run_policy_area($case, CASES);
            })+
        };
    }

    cases! {
        digest_stability => "digest-stability",
        enforcement_unsupported => "enforcement-unsupported",
        structural_conflict => "structural-conflict",
        dependency_conflict => "dependency-conflict",
        state_conflict => "state-conflict",
        interface_conflict => "interface-conflict",
        migration_missing_expiry => "migration-missing-expiry",
        migration_expired => "migration-expired",
        migration_expired_with_live_contract => "migration-expired-with-live-contract",
        override_without_authority => "override-without-authority",
    }
}

#[test]
fn inventory_unowned_acceptance() {
    run_case(&fixture_root().join("inventory/unowned-source-break-control"));
}

#[test]
fn snapshot_index_acceptance() {
    run_case(&fixture_root().join("snapshot/index-break-control"));
}

#[test]
fn inventory_expectation_rejects_a_disabled_unowned_control() {
    let case = fixture_root().join("inventory/unowned-source-break-control");
    let (expectation, mut observation) = execute_case(&case);
    assert_expectation(&case, &expectation, &observation);
    observation.document.as_mut().expect("inventory document")["summary"]["unowned_source"] =
        json!([]);
    let result = check_expectation(&expectation, &observation);
    assert!(
        result.is_err(),
        "the unowned-source case must fail when its guarded observation is removed"
    );
}

#[test]
fn snapshot_expectation_rejects_a_disabled_exclusion_control() {
    let case = fixture_root().join("snapshot/exclusions-break-control");
    let (expectation, mut observation) = execute_case(&case);
    assert_expectation(&case, &expectation, &observation);
    observation.document.as_mut().expect("inventory document")["summary"]["unread"] = json!([]);
    let result = check_expectation(&expectation, &observation);
    assert!(
        result.is_err(),
        "the exclusion case must fail when unread exclusions disappear"
    );
}

#[test]
fn snapshot_expectation_rejects_missing_exclusion_counts_and_submodules() {
    let case = fixture_root().join("snapshot/exclusions-break-control");
    let (expectation, mut observation) = execute_case(&case);
    assert_expectation(&case, &expectation, &observation);
    let original = observation.document.clone().expect("inventory document");
    for (pointer, value, diagnostic) in [
        ("/summary/ignored_files", json!(0), "ignored_files"),
        ("/summary/submodules", json!([]), "submodules"),
    ] {
        let mut document = original.clone();
        let field = document.pointer_mut(pointer).expect("observed facet");
        assert_ne!(*field, value, "mutation must change {pointer}");
        *field = value;
        observation.document = Some(document);
        let error = check_expectation(&expectation, &observation)
            .expect_err("missing exclusion facet must fail");
        assert!(error.contains(diagnostic), "{error}");
    }
}

#[test]
fn inventory_expectation_rejects_changed_class_module_and_count() {
    let case = fixture_root().join("inventory/colocated-test-break-control");
    let (expectation, mut observation) = execute_case(&case);
    assert_expectation(&case, &expectation, &observation);
    let original = observation.document.clone().expect("inventory document");
    let index = original["entries"]
        .as_array()
        .expect("inventory entries")
        .iter()
        .position(|entry| entry["path"] == "src/__tests__/helpers.ts")
        .expect("colocated test entry");
    for (pointer, value, diagnostic) in [
        (format!("/entries/{index}/class"), json!("source"), "class"),
        (format!("/entries/{index}/module"), Value::Null, "module"),
        ("/summary/by_class/test".into(), json!(0), "count"),
    ] {
        let mut document = original.clone();
        let field = document.pointer_mut(&pointer).expect("observed facet");
        assert_ne!(*field, value, "mutation must change {pointer}");
        *field = value;
        observation.document = Some(document);
        let error = check_expectation(&expectation, &observation)
            .expect_err("changed inventory facet must fail");
        assert!(error.contains(diagnostic), "{error}");
    }
}

#[test]
fn inventory_expectation_rejects_missing_unknown_files() {
    let case = fixture_root().join("inventory/unowned-source-negative");
    let (expectation, mut observation) = execute_case(&case);
    assert_expectation(&case, &expectation, &observation);
    let unknown =
        &mut observation.document.as_mut().expect("inventory document")["summary"]["unknown"];
    assert_ne!(*unknown, json!([]), "unknown fixture must be nonempty");
    *unknown = json!([]);
    let error =
        check_expectation(&expectation, &observation).expect_err("missing unknown files must fail");
    assert!(error.contains("unknown"), "{error}");
}

/// W0.6: the runner compares expect.json semantically and never byte-for-byte.
#[test]
fn semantic_comparison_ignores_key_order_at_every_depth() {
    let case = fixture_root().join("inventory/unowned-source-break-control");
    let original: Value =
        serde_json::from_slice(&fs::read(case.join("expect.json")).expect("read expect.json"))
            .expect("parse expect.json");
    let reordered = reverse_keys(&original);
    let reordered_bytes = serde_json::to_vec(&reordered).expect("encode reordered expectation");
    assert_ne!(
        serde_json::to_vec(&original).expect("encode expectation"),
        reordered_bytes,
        "reordering must change the serialized bytes, or this test proves nothing"
    );
    semantic_subset(&reordered, &original, "$").expect("reordered expectation still matches");

    // The same reordered file, parsed and compared through the runner's entry point.
    let (_, observation) = execute_case(&case);
    let expectation: Expectation =
        serde_json::from_slice(&reordered_bytes).expect("parse reordered expectation");
    check_expectation(&expectation, &observation).expect("reordered expectation passes");

    // A nested product document, reordered at every depth, is still the same document.
    let document = observation.document.expect("inventory document");
    let reordered_document = reverse_keys(&document);
    assert_ne!(
        serde_json::to_vec(&document).expect("encode document"),
        serde_json::to_vec(&reordered_document).expect("encode reordered document"),
        "reordering must change the serialized document bytes"
    );
    semantic_subset(&reordered_document, &document, "$").expect("reordered document still matches");
}

fn reverse_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .rev()
                .map(|(key, value)| (key.clone(), reverse_keys(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(reverse_keys).collect()),
        other => other.clone(),
    }
}

fn run_area(area: &str) {
    let root = fixture_root().join(area);
    let mut cases = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
        .map(|entry| entry.expect("fixture case").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    cases.sort();
    let expected: &[&str] = match area {
        "snapshot" => &[
            "case-collision-break-control",
            "case-collision-directory-break-control",
            "case-collision-negative",
            "case-collision-positive",
            "exclusions-break-control",
            "exclusions-negative",
            "exclusions-positive",
            "index-break-control",
            "index-negative",
            "index-positive",
        ],
        "inventory" => &[
            "classification-overlap-break-control",
            "classification-overlap-negative",
            "classification-overlap-positive",
            "colocated-test-break-control",
            "colocated-test-negative",
            "colocated-test-positive",
            "generated-absent-break-control",
            "generated-absent-negative",
            "generated-absent-positive",
            "generated-drift-absent-output",
            "generated-drift-break-control",
            "generated-drift-negative",
            "generated-drift-positive",
            "generated-ignored-break-control",
            "generated-ignored-negative",
            "generated-ignored-positive",
            "unowned-source-break-control",
            "unowned-source-negative",
            "unowned-source-positive",
        ],
        _ => panic!("unknown conformance area {area}"),
    };
    let actual = cases
        .iter()
        .map(|case| case.file_name().unwrap().to_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        expected.iter().copied().collect::<BTreeSet<_>>(),
        "{area} fixture census differs"
    );
    for case in cases {
        run_case(&case);
    }
}

fn run_policy_area(selected: &str, expected: &[&str]) {
    let root = fixture_root().join("policy");
    let mut cases = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
        .map(|entry| entry.expect("policy fixture case").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    cases.sort();
    let actual = cases
        .iter()
        .map(|case| case.file_name().unwrap().to_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        expected.iter().copied().collect::<BTreeSet<_>>(),
        "policy fixture census differs"
    );
    run_policy_case(&root.join(selected));
}

fn run_policy_case(case: &Path) {
    assert!(
        case.join("README.md").is_file(),
        "{} lacks README.md",
        case.display()
    );
    assert!(case.join("repo").is_dir(), "{} lacks repo/", case.display());
    assert!(
        case.join("warrant").is_dir(),
        "{} lacks warrant/",
        case.display()
    );
    let expectation: PolicyExpectation =
        serde_json::from_slice(&fs::read(case.join("expect.json")).expect("read expect.json"))
            .expect("parse policy expect.json");

    if expectation.equal_policy_digest.is_some() || expectation.source_layout_differs.is_some() {
        let original = run_policy_variant(case, "original");
        let reformatted = run_policy_variant(case, "reformatted");
        check_policy_observation(&expectation, &original)
            .unwrap_or_else(|reason| panic!("{} original: {reason}", case.display()));
        check_policy_observation(&expectation, &reformatted)
            .unwrap_or_else(|reason| panic!("{} reformatted: {reason}", case.display()));
        let original = effective_policy(&original);
        let reformatted = effective_policy(&reformatted);
        if let Some(expected) = expectation.equal_policy_digest {
            assert_eq!(
                original.policy_digest == reformatted.policy_digest,
                expected,
                "{} policy digest equality differs",
                case.display()
            );
        }
        if let Some(expected) = expectation.source_layout_differs {
            assert_eq!(
                original.sources != reformatted.sources,
                expected,
                "{} source layout comparison differs",
                case.display()
            );
        }
    } else {
        let observation = run_policy_fixture(case, None);
        check_policy_observation(&expectation, &observation)
            .unwrap_or_else(|reason| panic!("{}: {reason}", case.display()));
    }
}

fn run_policy_variant(case: &Path, variant: &str) -> Observation {
    run_policy_fixture(case, Some(variant))
}

fn run_policy_fixture(case: &Path, variant: Option<&str>) -> Observation {
    let temporary = tempfile::tempdir().expect("temporary policy conformance case");
    let repository = temporary.path().join("repo");
    copy_tree(&case.join("repo"), &repository);
    let source = variant
        .map(|variant| case.join("warrant").join(variant))
        .unwrap_or_else(|| case.join("warrant"));
    copy_tree(&source, &repository.join("warrant/policy"));
    initialize_repository(&repository, &[]);
    let compiled = run_cli(
        &repository,
        temporary.path(),
        &["policy".into(), "compile".into()],
    );
    let linted = run_cli(
        &repository,
        temporary.path(),
        &["policy".into(), "lint".into()],
    );
    assert_eq!(compiled.exit_code, linted.exit_code, "structural lint exit");
    assert_eq!(
        compiled.document, linted.document,
        "structural lint document"
    );
    compiled
}

fn check_policy_observation(
    expectation: &PolicyExpectation,
    observation: &Observation,
) -> Result<(), String> {
    if observation.exit_code != expectation.exit_code {
        return Err(format!(
            "exit code {}, expected {} (stderr: {})",
            observation.exit_code, expectation.exit_code, observation.stderr
        ));
    }
    if let Some(expected) = &expectation.error {
        let document = observation
            .document
            .as_ref()
            .ok_or("missing policy error document")?;
        if document["code"] != expected.code {
            return Err(format!(
                "error code was {}, expected {}",
                document["code"], expected.code
            ));
        }
        if let Some(needle) = &expected.reason_contains {
            let reason = document["reason"].as_str().unwrap_or_default();
            if !reason.contains(needle) {
                return Err(format!("error reason {reason:?} lacks {needle:?}"));
            }
        }
    }
    if let Some(expected) = &expectation.reports {
        let document = observation
            .document
            .as_ref()
            .ok_or("missing effective policy document")?;
        let reports = document["reports"]
            .as_array()
            .ok_or("effective policy lacks a reports array")?;
        let mut actual = reports
            .iter()
            .map(|report| {
                let contracts = report["contracts"]
                    .as_array()
                    .map(|ids| {
                        ids.iter()
                            .map(|id| id.as_str().unwrap_or_default().to_owned())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (
                    report["code"].as_str().unwrap_or_default().to_owned(),
                    contracts,
                )
            })
            .collect::<Vec<_>>();
        let mut wanted = expected
            .iter()
            .map(|report| (report.code.clone(), report.contracts.clone()))
            .collect::<Vec<_>>();
        actual.sort();
        wanted.sort();
        if actual != wanted {
            return Err(format!("reports were {actual:?}, expected {wanted:?}"));
        }
    }
    if let Some(expected) = &expectation.contract_ids {
        let document = observation
            .document
            .as_ref()
            .ok_or("missing effective policy document")?;
        let mut actual = document["contracts"]
            .as_array()
            .ok_or("effective policy lacks a contracts array")?
            .iter()
            .map(|contract| contract["id"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        let mut wanted = expected.clone();
        actual.sort();
        wanted.sort();
        if actual != wanted {
            return Err(format!("contract ids were {actual:?}, expected {wanted:?}"));
        }
    }
    Ok(())
}

fn effective_policy(observation: &Observation) -> EffectivePolicy {
    serde_json::from_value(
        observation
            .document
            .clone()
            .expect("policy compile must print an effective policy"),
    )
    .expect("typed effective policy")
}

fn run_case(case: &Path) {
    let (expectation, observation) = execute_case(case);
    assert_expectation(case, &expectation, &observation);
}

fn execute_case(case: &Path) -> (Expectation, Observation) {
    assert!(
        case.join("README.md").is_file(),
        "{} lacks README.md",
        case.display()
    );
    assert!(case.join("repo").is_dir(), "{} lacks repo/", case.display());
    let expectation: Expectation =
        serde_json::from_slice(&fs::read(case.join("expect.json")).expect("read expect.json"))
            .expect("parse expect.json");
    let temporary = tempfile::tempdir().expect("temporary conformance case");
    let repository = temporary.path().join("repo");
    copy_tree(&case.join("repo"), &repository);
    if case.join("warrant").is_dir() {
        copy_tree(&case.join("warrant"), &repository.join("warrant"));
    }
    initialize_repository(&repository, &expectation.setup.force_add);
    apply_setup(&repository, temporary.path(), &expectation.setup);
    let observation = match expectation.driver {
        Driver::Cli => run_cli(&repository, temporary.path(), &expectation.args),
        Driver::InventoryApi => run_inventory_api(&repository, &expectation),
    };
    (expectation, observation)
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture directory");
    let mut entries = fs::read_dir(source)
        .unwrap_or_else(|error| panic!("read {}: {error}", source.display()))
        .collect::<Result<Vec<_>, _>>()
        .expect("read fixture entries");
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let target = destination.join(entry.file_name());
        let kind = entry.file_type().expect("fixture file type");
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else if kind.is_symlink() {
            let link = fs::read_link(entry.path()).expect("fixture symlink");
            symlink(link, target).expect("copy fixture symlink");
        } else {
            fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}

fn initialize_repository(repository: &Path, force_add: &[String]) {
    git(repository, &["init", "-q"]);
    git(repository, &["add", "."]);
    for path in force_add {
        git(repository, &["add", "-f", "--", path]);
    }
    git(
        repository,
        &[
            "-c",
            "user.name=Warrant Conformance",
            "-c",
            "user.email=warrant@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
}

fn apply_setup(repository: &Path, temporary: &Path, setup: &Setup) {
    for (path, contents) in &setup.stage_files {
        write_file(repository, path, contents.as_bytes());
        git(repository, &["add", "--", path]);
    }
    for (path, contents) in &setup.worktree_files {
        write_file(repository, path, contents.as_bytes());
    }
    for (path, contents) in &setup.case_variant_files {
        write_file(repository, path, contents.as_bytes());
        let blob = hash_object(repository, contents.as_bytes());
        git(
            repository,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob},{path}"),
            ],
        );
    }
    for (path, target) in &setup.symlinks {
        let link = repository.join(path);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).expect("create symlink parent");
        }
        symlink(target, link).expect("create fixture symlink");
    }
    if let Some(path) = &setup.local_submodule {
        let source = temporary.join("submodule-source");
        fs::create_dir_all(&source).expect("create submodule source");
        write_file(&source, "member.txt", b"submodule\n");
        git(&source, &["init", "-q"]);
        git(&source, &["add", "member.txt"]);
        git(
            &source,
            &[
                "-c",
                "user.name=Warrant Conformance",
                "-c",
                "user.email=warrant@example.invalid",
                "commit",
                "-qm",
                "submodule fixture",
            ],
        );
        let mut command = Command::new("git");
        command
            .args(["-c", "protocol.file.allow=always", "submodule", "add", "-q"])
            .arg(&source)
            .arg(path)
            .current_dir(repository);
        neutralize_git_environment(&mut command);
        let output = command.output().expect("add local submodule");
        assert!(
            output.status.success(),
            "git submodule add: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Write `contents` as a blob without reading any worktree path.
fn hash_object(repository: &Path, contents: &[u8]) -> String {
    use std::io::Write as _;
    let mut command = Command::new("git");
    command
        .args(["hash-object", "-w", "--stdin"])
        .current_dir(repository)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    neutralize_git_environment(&mut command);
    let mut child = command.spawn().expect("run git hash-object");
    child
        .stdin
        .take()
        .expect("hash-object stdin")
        .write_all(contents)
        .expect("write blob contents");
    let output = child.wait_with_output().expect("wait for git hash-object");
    assert!(
        output.status.success(),
        "git hash-object: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git UTF-8")
        .trim()
        .to_owned()
}

fn write_file(root: &Path, path: &str, contents: &[u8]) {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("create fixture file parent");
    }
    fs::write(target, contents).expect("write fixture file");
}

fn run_cli(repository: &Path, cache: &Path, args: &[String]) -> Observation {
    let mut command = Command::new(env!("CARGO_BIN_EXE_warrant"));
    command
        .args(args)
        .current_dir(repository)
        .env("XDG_CACHE_HOME", cache.join("cache"));
    neutralize_git_environment(&mut command);
    // The oracles are Git's answers before Warrant runs, so a run that rewrote the index
    // could not move the oracle along with its own answer. `write-tree` may store a
    // cache tree in the index, so the bytes are read after the oracles are taken.
    let format = git(repository, &["rev-parse", "--show-object-format"]);
    let index_tree = format!("{format}:{}", git(repository, &["write-tree"]));
    let git_ignored = git(
        repository,
        &["ls-files", "--others", "--ignored", "--exclude-standard"],
    )
    .lines()
    .map(str::to_owned)
    .collect();
    let index_path = repository.join(git(repository, &["rev-parse", "--git-path", "index"]));
    let index_before = fs::read(&index_path).expect("read index before warrant");
    let output = command.output().expect("run warrant");
    assert!(
        fs::read(&index_path).expect("read index after warrant") == index_before,
        "warrant {args:?} changed the index file in {}",
        repository.display()
    );
    let mut observation = output_observation(output);
    observation.index_tree = Some(index_tree);
    observation.git_ignored = Some(git_ignored);
    observation
}

fn output_observation(output: Output) -> Observation {
    let exit_code = output.status.code().expect("warrant did not exit normally");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    // A finding-bearing document exits 1 with the document on stdout (spec 10.2); an
    // error writes nothing there and its document goes to stderr.
    let bytes = if output.status.success() || !output.stdout.is_empty() {
        &output.stdout
    } else {
        stderr.as_bytes()
    };
    let document = if bytes.is_empty() {
        None
    } else {
        Some(serde_json::from_slice(bytes).expect("warrant JSON document"))
    };
    Observation {
        exit_code,
        document,
        stderr,
        index_tree: None,
        git_ignored: None,
    }
}

fn run_inventory_api(repository: &Path, expectation: &Expectation) -> Observation {
    let manifest = load_manifest(repository);
    let config = BuildConfig {
        modules: expectation
            .modules
            .iter()
            .map(|module| ModuleSelector {
                id: module.id.clone(),
                files: module.files.clone(),
            })
            .collect(),
        class_rules: expectation
            .class_rules
            .iter()
            .map(|rule| ClassRule {
                id: rule.id.clone(),
                class: rule.class,
                files: rule.files.clone(),
                replaces: rule.replaces,
            })
            .collect(),
    };
    match warrant_snapshot::capture(
        repository,
        SnapshotKind::Worktree,
        None,
        &manifest.snapshot,
        |snapshot| {
            let read = |path: &str| {
                snapshot
                    .read(path)
                    .map_err(|error| warrant_inventory::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            let resolve = |path: &str| {
                snapshot
                    .resolve(path)
                    .map_err(|error| warrant_inventory::ReadError {
                        code: error.document.code,
                        reason: error.document.reason,
                    })
            };
            warrant_inventory::build(
                repository,
                warrant_inventory::CapturedSnapshot {
                    manifest: snapshot.manifest(),
                    entries: snapshot.entries(),
                    read: &read,
                    resolve: &resolve,
                    untracked: snapshot.untracked(),
                },
                &manifest,
                &config,
            )
            .map_err(|error| warrant_snapshot::SnapshotError {
                document: warrant_core::nouns::ErrorDocument {
                    schema_version: "warrant.error/1".into(),
                    code: error.code().into(),
                    reason: error.to_string(),
                    locations: Vec::new(),
                    next_diagnostic: None,
                },
            })
        },
    ) {
        Ok((_, built)) => Observation {
            exit_code: 0,
            document: Some(serde_json::to_value(built.document).expect("inventory JSON")),
            stderr: String::new(),
            index_tree: None,
            git_ignored: None,
        },
        Err(error) => {
            let stderr = error.to_string();
            Observation {
                exit_code: 2,
                document: Some(serde_json::to_value(error.document).expect("error JSON")),
                stderr,
                index_tree: None,
                git_ignored: None,
            }
        }
    }
}

fn load_manifest(repository: &Path) -> WarrantManifest {
    let path = repository.join("warrant/warrant.yaml");
    let source =
        fs::read_to_string(path).unwrap_or_else(|_| "schema_version: warrant.manifest/1\n".into());
    WarrantManifest::parse(&source).expect("fixture manifest")
}

fn assert_expectation(case: &Path, expectation: &Expectation, observation: &Observation) {
    if let Err(reason) = check_expectation(expectation, observation) {
        panic!("{}: {reason}; observation={observation:#?}", case.display());
    }
}

fn check_expectation(expectation: &Expectation, observation: &Observation) -> Result<(), String> {
    if observation.exit_code != expectation.exit_code {
        return Err(format!(
            "exit code {}, expected {} (stderr: {})",
            observation.exit_code, expectation.exit_code, observation.stderr
        ));
    }
    if let Some(expected) = &expectation.document {
        semantic_subset(
            expected,
            observation
                .document
                .as_ref()
                .ok_or("missing JSON document")?,
            "$",
        )?;
    }
    if let Some(expected) = &expectation.error {
        let document = observation
            .document
            .as_ref()
            .ok_or("missing error document")?;
        if document["code"] != expected.code {
            return Err(format!(
                "error code was {}, expected {}",
                document["code"], expected.code
            ));
        }
        if let Some(needle) = &expected.reason_contains {
            let reason = document["reason"].as_str().unwrap_or_default();
            if !reason.contains(needle) {
                return Err(format!("error reason {reason:?} lacks {needle:?}"));
            }
        }
    }
    if let Some(expected) = &expectation.inventory {
        check_inventory(
            expected,
            observation.document.as_ref().ok_or("missing inventory")?,
        )?;
    }
    if let Some(expected) = &expectation.git_ignored {
        let actual = observation
            .git_ignored
            .as_ref()
            .ok_or("git ignored listing was not observed")?;
        compare_sorted("git_ignored", actual, expected)?;
    }
    if let Some(should_match) = expectation.tree_matches_index {
        let document = observation.document.as_ref().ok_or("missing snapshot")?;
        let tree = document["tree"]
            .as_str()
            .ok_or("snapshot tree is not a string")?;
        let matches = observation.index_tree.as_deref() == Some(tree);
        if matches != should_match {
            return Err(format!(
                "snapshot tree {tree} match-index was {matches}, expected {should_match}"
            ));
        }
    }
    Ok(())
}

fn check_inventory(expected: &InventoryExpect, actual: &Value) -> Result<(), String> {
    let document: InventoryDocument =
        serde_json::from_value(actual.clone()).map_err(|error| error.to_string())?;
    let entries = document
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for (path, class) in &expected.classes {
        let entry = entries
            .get(path.as_str())
            .ok_or_else(|| format!("missing inventory entry {path}"))?;
        if entry.class != *class {
            return Err(format!(
                "{path} class {:?}, expected {class:?}",
                entry.class
            ));
        }
    }
    for (path, module) in &expected.modules {
        let entry = entries
            .get(path.as_str())
            .ok_or_else(|| format!("missing inventory entry {path}"))?;
        if &entry.module != module {
            return Err(format!(
                "{path} module {:?}, expected {module:?}",
                entry.module
            ));
        }
    }
    for (path, reason) in &expected.reasons {
        let entry = entries
            .get(path.as_str())
            .ok_or_else(|| format!("missing inventory entry {path}"))?;
        if &entry.reason != reason {
            return Err(format!(
                "{path} reason {:?}, expected {reason:?}",
                entry.reason
            ));
        }
    }
    for (class, count) in &expected.by_class {
        if document.summary.by_class.get(class).copied().unwrap_or(0) != *count {
            return Err(format!("class {class} count differs from {count}"));
        }
    }
    if let Some(expected) = &expected.unowned_source {
        compare_sorted("unowned_source", &document.summary.unowned_source, expected)?;
    }
    if let Some(expected) = &expected.unknown {
        compare_sorted("unknown", &document.summary.unknown, expected)?;
    }
    if let Some(expected) = &expected.generated_absent {
        let actual = document
            .summary
            .generated_absent
            .iter()
            .map(|entry| entry.declaration.clone())
            .collect::<Vec<_>>();
        compare_sorted("generated_absent", &actual, expected)?;
    }
    for (declaration, producer) in &expected.generated_absent_producers {
        let row = document
            .summary
            .generated_absent
            .iter()
            .find(|row| &row.declaration == declaration)
            .ok_or_else(|| format!("generated_absent lacks {declaration}"))?;
        if &row.producer != producer {
            return Err(format!(
                "generated_absent {declaration} producer {:?}, expected {producer:?}",
                row.producer
            ));
        }
    }
    if let Some(expected) = &expected.generated_drift {
        let mut actual = document
            .summary
            .generated_drift
            .clone()
            .ok_or("generated producers were not re-run (generated_drift is null)")?;
        let mut expected = expected.clone();
        actual.sort_by(|left, right| {
            (&left.path, &left.producer).cmp(&(&right.path, &right.producer))
        });
        expected.sort_by(|left, right| {
            (&left.path, &left.producer).cmp(&(&right.path, &right.producer))
        });
        if actual != expected {
            return Err(format!("generated_drift {actual:?}, expected {expected:?}"));
        }
    }
    if let Some(expected) = &expected.unread {
        let actual = document
            .summary
            .unread
            .iter()
            .map(|entry| (entry.path.clone(), entry.reason.clone()))
            .collect::<BTreeMap<_, _>>();
        if &actual != expected {
            return Err(format!("unread {actual:?}, expected {expected:?}"));
        }
    }
    if let Some(expected) = expected.ignored_files
        && document.summary.ignored_files != Some(expected)
    {
        return Err(format!(
            "ignored_files {:?}, expected {expected}",
            document.summary.ignored_files
        ));
    }
    if let Some(expected) = &expected.submodules {
        let actual = document
            .summary
            .submodules
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        compare_sorted("submodules", &actual, expected)?;
    }
    Ok(())
}

fn compare_sorted(label: &str, actual: &[String], expected: &[String]) -> Result<(), String> {
    let mut actual = actual.to_vec();
    let mut expected = expected.to_vec();
    actual.sort();
    expected.sort();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} {actual:?}, expected {expected:?}"))
    }
}

fn semantic_subset(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                let next = format!("{path}.{key}");
                semantic_subset(
                    value,
                    actual.get(key).ok_or_else(|| format!("missing {next}"))?,
                    &next,
                )?;
            }
            Ok(())
        }
        (Value::Array(expected), Value::Array(actual)) => {
            let mut expected = expected.clone();
            let mut actual = actual.clone();
            sort_json(&mut expected);
            sort_json(&mut actual);
            if expected == actual {
                Ok(())
            } else {
                Err(format!("{path}: {actual:?}, expected {expected:?}"))
            }
        }
        _ if expected == actual => Ok(()),
        _ => Err(format!("{path}: {actual:?}, expected {expected:?}")),
    }
}

/// Sort array elements by a key-order-independent rendering: `Value::to_string` keeps
/// object insertion order, so the same objects with reordered keys would sort apart.
fn sort_json(values: &mut [Value]) {
    values.sort_by_cached_key(|value| {
        let mut canonical = value.clone();
        canonical.sort_all_objects();
        canonical.to_string()
    });
}

fn git(repository: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.args(args).current_dir(repository);
    neutralize_git_environment(&mut command);
    let output = command.output().expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git UTF-8")
        .trim()
        .to_owned()
}

fn neutralize_git_environment(command: &mut Command) {
    static EMPTY_EXCLUDES: OnceLock<tempfile::NamedTempFile> = OnceLock::new();
    let excludes = EMPTY_EXCLUDES
        .get_or_init(|| tempfile::NamedTempFile::new().expect("create empty Git excludes file"));

    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.excludesFile")
        .env("GIT_CONFIG_VALUE_0", excludes.path());
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(name);
    }
}
