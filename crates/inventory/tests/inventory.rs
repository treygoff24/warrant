use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::tempdir;
use warrant_core::manifest::WarrantManifest;
use warrant_core::nouns::InventoryClass;
use warrant_inventory::{
    BuildConfig, ClassRule, DeliberateExclusion, GeneratedIssueCode, InventoryError,
    ModuleSelector, build, discover_units, lint_ownership, verify_generated,
};

fn write(root: &Path, path: &str, contents: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().expect("file parent")).expect("create parent");
    fs::write(target, contents).expect("write fixture file");
}

fn manifest(inventory: &str) -> WarrantManifest {
    WarrantManifest::parse(&format!(
        r#"
schema_version: warrant.manifest/1
integrations:
  lang-ts:
    enabled: true
  lang-rust:
    enabled: true
inventory:
{inventory}
"#
    ))
    .expect("valid fixture manifest")
}

fn empty_manifest() -> WarrantManifest {
    manifest("  unknown: report")
}

#[test]
fn overlapping_module_selectors_are_an_error() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/shared.ts", "export const value = 1;\n");

    let modules = [
        ModuleSelector {
            id: "first".into(),
            files: vec!["src/**".into()],
        },
        ModuleSelector {
            id: "second".into(),
            files: vec!["src/shared.ts".into()],
        },
    ];
    let error = lint_ownership(root.path(), &modules)
        .expect_err("overlap must not be resolved by declaration order");

    assert!(matches!(
        error,
        InventoryError::OwnershipOverlap { path, modules }
            if path == "src/shared.ts" && modules == ["first", "second"]
    ));
}

#[test]
fn classification_defaults_and_provenance_cover_every_class() {
    let root = tempdir().expect("temporary repository");
    for (path, contents) in [
        ("src/main.ts", "export {};\n"),
        ("src/main.test.ts", "test('x', () => {});\n"),
        ("package.json", "{}\n"),
        ("scripts/release.sh", "exit 0\n"),
        ("migrations/001.sql", "select 1;\n"),
        ("types/api.d.ts", "export {};\n"),
        ("generated/client.ts", "generated\n"),
        ("vendor/library.ts", "vendored\n"),
        ("public/logo.png", "image\n"),
        ("README.md", "docs\n"),
        ("dist/app.js", "built\n"),
        ("legacy.pl", "unknown\n"),
    ] {
        write(root.path(), path, contents);
    }
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/**"]
      producer: "generate-client"
      reproducible: false
  vendored:
    - files: ["vendor/**"]
      source: "https://example.invalid/library"
      version: "1.2.3"
      treatment: "checked-as-third-party"
"#,
    );
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "application".into(),
            files: vec!["src/**".into()],
        }],
        deliberate_exclusions: vec![
            DeliberateExclusion {
                path: "third-party/old".into(),
                class: InventoryClass::Submodule,
                reason: "submodule-not-descended".into(),
                blob: Some("sha1:abc".into()),
            },
            DeliberateExclusion {
                path: "ignored/cache.bin".into(),
                class: InventoryClass::Ignored,
                reason: "gitignored".into(),
                blob: None,
            },
            DeliberateExclusion {
                path: "large/data.bin".into(),
                class: InventoryClass::Unread,
                reason: "oversize".into(),
                blob: None,
            },
        ],
        ignored_files: Some(1),
        ..BuildConfig::default()
    };

    let built = build(root.path(), &manifest, &config).expect("inventory builds");
    let classes: BTreeMap<_, _> = built
        .document
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.class))
        .collect();
    assert_eq!(classes["src/main.ts"], InventoryClass::Source);
    assert_eq!(classes["src/main.test.ts"], InventoryClass::Test);
    assert_eq!(classes["package.json"], InventoryClass::Config);
    assert_eq!(classes["scripts/release.sh"], InventoryClass::Script);
    assert_eq!(classes["migrations/001.sql"], InventoryClass::Migration);
    assert_eq!(classes["types/api.d.ts"], InventoryClass::Schema);
    assert_eq!(classes["generated/client.ts"], InventoryClass::Generated);
    assert_eq!(classes["vendor/library.ts"], InventoryClass::Vendored);
    assert_eq!(classes["public/logo.png"], InventoryClass::Asset);
    assert_eq!(classes["README.md"], InventoryClass::Doc);
    assert_eq!(classes["dist/app.js"], InventoryClass::BuildOutput);
    assert_eq!(classes["third-party/old"], InventoryClass::Submodule);
    assert_eq!(classes["ignored/cache.bin"], InventoryClass::Ignored);
    assert_eq!(classes["legacy.pl"], InventoryClass::Unknown);
    assert_eq!(classes["large/data.bin"], InventoryClass::Unread);

    let generated = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "generated/client.ts")
        .expect("generated entry");
    assert_eq!(
        generated
            .generated_by
            .as_ref()
            .expect("generated provenance")
            .producer,
        "generate-client"
    );
    let vendored = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "vendor/library.ts")
        .expect("vendored entry");
    assert_eq!(
        vendored
            .vendored_from
            .as_ref()
            .expect("vendored provenance")
            .version,
        "1.2.3"
    );
}

#[test]
fn colocated_test_keeps_its_class_and_owner() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/service.test.ts", "export {};\n");
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "service".into(),
            files: vec!["src/**".into()],
        }],
        ..BuildConfig::default()
    };

    let built = build(root.path(), &empty_manifest(), &config).expect("inventory builds");
    let entry = &built.document.entries[0];
    assert_eq!(entry.class, InventoryClass::Test);
    assert_eq!(entry.module.as_deref(), Some("service"));
}

#[test]
fn first_party_source_outside_module_is_unowned() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "outside.ts", "export {};\n");

    let built =
        build(root.path(), &empty_manifest(), &BuildConfig::default()).expect("inventory builds");
    assert_eq!(built.document.entries[0].module, None);
    assert_eq!(built.document.summary.unowned_source, ["outside.ts"]);
}

#[test]
fn source_defaults_only_apply_for_enabled_integrations() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "types/disabled.d.ts", "export {};\n");
    let manifest =
        WarrantManifest::parse("schema_version: warrant.manifest/1\n").expect("valid manifest");

    let built = build(root.path(), &manifest, &BuildConfig::default()).expect("inventory builds");
    assert_eq!(built.document.entries[0].class, InventoryClass::Unknown);
}

#[test]
fn conflicting_class_rules_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let first = ClassRule {
        id: "doc-rule".into(),
        class: InventoryClass::Doc,
        files: vec!["src/**".into()],
        replaces: Some(InventoryClass::Source),
    };
    let second = ClassRule {
        id: "script-rule".into(),
        class: InventoryClass::Script,
        files: vec!["src/value.ts".into()],
        replaces: Some(InventoryClass::Source),
    };

    for rules in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        let error = build(
            root.path(),
            &empty_manifest(),
            &BuildConfig {
                class_rules: rules,
                ..BuildConfig::default()
            },
        )
        .expect_err("conflicting classes must fail");
        assert!(matches!(
            error,
            InventoryError::ClassificationConflict { path, rules }
                if path == "src/value.ts" && rules == ["doc-rule", "script-rule"]
        ));
    }
}

#[test]
fn explicit_override_must_name_the_default_it_replaces() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let make_config = |replaces| BuildConfig {
        class_rules: vec![ClassRule {
            id: "migration".into(),
            class: InventoryClass::Migration,
            files: vec!["src/value.ts".into()],
            replaces,
        }],
        ..BuildConfig::default()
    };

    assert!(matches!(
        build(root.path(), &empty_manifest(), &make_config(None)),
        Err(InventoryError::MissingDefaultReplacement {
            default: InventoryClass::Source,
            ..
        })
    ));
    assert!(matches!(
        build(
            root.path(),
            &empty_manifest(),
            &make_config(Some(InventoryClass::Test))
        ),
        Err(InventoryError::WrongDefaultReplacement {
            expected: InventoryClass::Source,
            named: InventoryClass::Test,
            ..
        })
    ));
    let built = build(
        root.path(),
        &empty_manifest(),
        &make_config(Some(InventoryClass::Source)),
    )
    .expect("named replacement is valid");
    assert_eq!(built.document.entries[0].class, InventoryClass::Migration);
}

#[test]
fn discovers_monorepo_units_from_all_declared_sources() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "package.json",
        r#"{"private":true,"workspaces":["packages/*"]}"#,
    );
    write(root.path(), "tsconfig.json", "{}\n");
    write(
        root.path(),
        "packages/web/package.json",
        r#"{"name":"web"}"#,
    );
    write(root.path(), "packages/web/tsconfig.json", "{}\n");
    write(root.path(), "packages/web/src/index.ts", "export {};\n");
    write(
        root.path(),
        "pnpm-workspace.yaml",
        "packages:\n  - 'services/*'\n",
    );
    write(
        root.path(),
        "services/api/package.json",
        r#"{"name":"api"}"#,
    );
    write(root.path(), "services/api/src/index.ts", "export {};\n");
    write(
        root.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/tool\"]\nresolver = \"3\"\n",
    );
    write(
        root.path(),
        "crates/tool/Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(root.path(), "crates/tool/src/lib.rs", "pub fn tool() {}\n");

    let units = discover_units(root.path()).expect("units discovered");
    let roots: Vec<_> = units.iter().map(|unit| unit.root.as_str()).collect();
    assert!(roots.contains(&"."));
    assert!(roots.contains(&"packages/web"));
    assert!(roots.contains(&"services/api"));
    assert!(roots.contains(&"crates/tool"));

    let built =
        build(root.path(), &empty_manifest(), &BuildConfig::default()).expect("inventory builds");
    let unit_of = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.unit.as_deref())
    };
    assert_eq!(unit_of("packages/web/src/index.ts"), Some("packages/web"));
    assert_eq!(unit_of("services/api/src/index.ts"), Some("services/api"));
    assert_eq!(unit_of("crates/tool/src/lib.rs"), Some("crates/tool"));
}

#[test]
fn discovers_package_and_manifest_entrypoints() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "package.json",
        r#"{"bin":"./bin/cli.js","main":"./src/main.js","module":"./src/module.js","exports":{".":"./src/export.js"}}"#,
    );
    for path in [
        "bin/cli.js",
        "src/main.js",
        "src/module.js",
        "src/export.js",
        "src/job.ts",
    ] {
        write(root.path(), path, "export {};\n");
    }
    let manifest = WarrantManifest::parse(
        r#"
schema_version: warrant.manifest/1
integrations:
  lang-ts:
    enabled: true
entrypoints:
  - kind: workflow
    path: src/job.ts
    symbol: run
"#,
    )
    .expect("valid manifest");

    let built = build(root.path(), &manifest, &BuildConfig::default()).expect("inventory builds");
    let kinds = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("entrypoint path")
            .entrypoints
            .iter()
            .map(|entrypoint| entrypoint.kind.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds("bin/cli.js"), ["package-bin"]);
    assert_eq!(kinds("src/main.js"), ["package-main"]);
    assert_eq!(kinds("src/module.js"), ["package-main"]);
    assert_eq!(kinds("src/export.js"), ["package-export"]);
    assert_eq!(kinds("src/job.ts"), ["workflow"]);
}

#[test]
fn generated_verification_reports_drift_and_absence() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/value.txt", "stale\n");
    write(root.path(), "source.txt", "input\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/value.txt", "generated/missing.txt"]
      producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt"
      reproducible: true
"#,
    );

    let built = build(root.path(), &manifest, &BuildConfig::default())
        .expect("inventory records declared missing output");
    let absent = built
        .document
        .entries
        .iter()
        .find(|entry| entry.path == "generated/missing.txt")
        .expect("absent generated path has an inventory entry");
    assert_eq!(absent.class, InventoryClass::Generated);
    assert_eq!(absent.reason, "generated-absent");
    assert_eq!(
        absent
            .generated_by
            .as_ref()
            .expect("producer provenance")
            .producer,
        "mkdir -p generated && printf 'fresh\\n' > generated/value.txt"
    );

    let issues = verify_generated(root.path(), &manifest).expect("producer runs");
    assert_eq!(
        issues,
        [
            warrant_inventory::GeneratedIssue {
                code: GeneratedIssueCode::GeneratedAbsent,
                path: "generated/missing.txt".into(),
                producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt".into(),
            },
            warrant_inventory::GeneratedIssue {
                code: GeneratedIssueCode::GeneratedDrift,
                path: "generated/value.txt".into(),
                producer: "mkdir -p generated && printf 'fresh\\n' > generated/value.txt".into(),
            },
        ]
    );
}

#[test]
fn completeness_counts_equal_entries_and_digest_is_stable() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/owned.ts", "export {};\n");
    write(root.path(), "src/unowned.ts", "export {};\n");
    write(root.path(), "mystery.xyz", "?\n");
    let config = BuildConfig {
        modules: vec![ModuleSelector {
            id: "owned".into(),
            files: vec!["src/owned.ts".into()],
        }],
        deliberate_exclusions: vec![DeliberateExclusion {
            path: "external".into(),
            class: InventoryClass::Submodule,
            reason: "not descended".into(),
            blob: Some("sha1:123".into()),
        }],
        ..BuildConfig::default()
    };

    let first = build(root.path(), &empty_manifest(), &config).expect("first inventory");
    let second = build(root.path(), &empty_manifest(), &config).expect("second inventory");
    let counted: u64 = first.document.summary.by_class.values().sum();
    assert_eq!(counted, first.document.entries.len() as u64);
    assert_eq!(first.document.summary.files, counted);
    assert_eq!(first.document.summary.unowned_source, ["src/unowned.ts"]);
    assert_eq!(first.document.summary.unknown, ["mystery.xyz"]);
    assert_eq!(first.document.summary.submodules[0].path, "external");
    assert_eq!(first.digest, second.digest);
}
