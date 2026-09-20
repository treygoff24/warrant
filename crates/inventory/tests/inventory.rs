use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::tempdir;
use warrant_core::manifest::WarrantManifest;
use warrant_core::nouns::{InventoryClass, InventoryEntry};
use warrant_inventory::{
    BuildConfig, ClassRule, GeneratedIssueCode, InventoryError, ModuleSelector, build,
    discover_units, lint_ownership, verify_generated,
};

fn write(root: &Path, path: &str, contents: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().expect("file parent")).expect("create parent");
    fs::write(target, contents).expect("write fixture file");
}

fn snapshot_entry(path: &str, class: InventoryClass, blob: Option<&str>) -> InventoryEntry {
    InventoryEntry {
        path: path.into(),
        blob: blob.map(str::to_owned),
        class,
        language: None,
        unit: None,
        module: None,
        by: "snapshot".into(),
        reason: "captured fixture".into(),
        entrypoints: Vec::new(),
        unread: (class == InventoryClass::Unread).then(|| "oversize".into()),
        generated_by: None,
        vendored_from: None,
    }
}

fn snapshot(root: &Path) -> Vec<InventoryEntry> {
    fn collect(root: &Path, directory: &Path, entries: &mut Vec<InventoryEntry>) {
        for item in fs::read_dir(directory).expect("read fixture directory") {
            let item = item.expect("read fixture entry");
            let path = item.path();
            if item.file_type().expect("fixture file type").is_dir() {
                collect(root, &path, entries);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("fixture path below root")
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push(snapshot_entry(
                    &relative,
                    InventoryClass::Unknown,
                    Some(&format!("sha1:fixture-{relative}")),
                ));
            }
        }
    }

    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
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
fn nested_repository_is_rejected() {
    for git_is_file in [false, true] {
        let root = tempdir().expect("temporary repository");
        if git_is_file {
            write(root.path(), "vendor/nested/.git", "gitdir: elsewhere\n");
        } else {
            write(
                root.path(),
                "vendor/nested/.git/HEAD",
                "ref: refs/heads/main\n",
            );
        }
        write(root.path(), "vendor/nested/src/b.ts", "export {};\n");
        let error = build(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &BuildConfig::default(),
        )
        .expect_err("nested repository has two identities");
        assert!(
            matches!(error, InventoryError::NestedRepository { path } if path == "vendor/nested")
        );
    }
}

#[test]
fn declared_submodule_contents_are_not_first_party() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(
        root.path(),
        "vendor/sub/.git/HEAD",
        "ref: refs/heads/main\n",
    );
    write(root.path(), "vendor/sub/src/b.ts", "export {};\n");
    write(
        root.path(),
        "vendor/sub/package.json",
        "not first-party config",
    );
    write(root.path(), "vendor/submarine.ts", "export {};\n");
    let mut listing = snapshot(root.path());
    listing.push(snapshot_entry(
        "vendor/sub",
        InventoryClass::Submodule,
        Some("sha1:abc"),
    ));
    let built = build(
        root.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("declared submodule is allowed");
    let paths: Vec<_> = built
        .document
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(paths, ["vendor/sub", "vendor/submarine.ts"]);
    assert_eq!(built.document.summary.submodules[0].path, "vendor/sub");
    assert_eq!(
        built.document.summary.unowned_source,
        ["vendor/submarine.ts"]
    );
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
        ..BuildConfig::default()
    };
    let mut listing = snapshot(root.path());
    listing.extend([
        snapshot_entry(
            "third-party/old",
            InventoryClass::Submodule,
            Some("sha1:abc"),
        ),
        snapshot_entry("ignored/cache.bin", InventoryClass::Ignored, None),
        snapshot_entry("large/data.bin", InventoryClass::Unread, None),
    ]);

    let built = build(root.path(), &listing, &manifest, &config).expect("inventory builds");
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

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &config,
    )
    .expect("inventory builds");
    let entry = &built.document.entries[0];
    assert_eq!(entry.class, InventoryClass::Test);
    assert_eq!(entry.module.as_deref(), Some("service"));
}

#[test]
fn first_party_source_outside_module_is_unowned() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "outside.ts", "export {};\n");

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    assert_eq!(built.document.entries[0].module, None);
    assert_eq!(built.document.entries[0].unit.as_deref(), Some("."));
    assert_eq!(built.document.entries[0].by, "implicit-root-unit");
    assert_eq!(built.document.summary.unowned_source, ["outside.ts"]);
    assert_eq!(built.document.summary.unit_aliases.len(), 1);
    assert_eq!(built.document.summary.unit_aliases[0].unit, ".");
    assert_eq!(built.document.summary.unit_aliases[0].alias_table, None);
    assert_eq!(
        built.document.summary.unit_aliases[0].by,
        "implicit-root-fallback"
    );
}

#[test]
fn source_defaults_only_apply_for_enabled_integrations() {
    let root = tempdir().expect("temporary repository");
    for path in [
        "types/disabled.d.ts",
        "src/value.test.ts",
        "src/value.spec.tsx",
        "__tests__/value.ts",
    ] {
        write(root.path(), path, "export {};\n");
    }
    let manifest =
        WarrantManifest::parse("schema_version: warrant.manifest/1\n").expect("valid manifest");

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    for entry in &built.document.entries {
        assert_eq!(entry.class, InventoryClass::Unknown, "{}", entry.path);
    }
}

#[test]
fn unowned_bin_source_keeps_integration_defaults() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "package.json", "{}");
    let sources = [
        "crates/tool/src/bin/helper.rs",
        "migrations/change.ts",
        "schemas/model.rs",
        "scripts/tool.ts",
        "src/bin/cli.ts",
        "tests/helper.rs",
        "vite.config.ts",
    ];
    for path in sources {
        write(root.path(), path, "// source\n");
    }
    write(root.path(), "scripts/release.sh", "exit 0\n");
    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    for path in sources {
        let entry = built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("source entry");
        assert_eq!(entry.class, InventoryClass::Source, "{path}");
        assert_eq!(entry.module, None);
        assert_eq!(entry.by, "default:source");
    }
    assert_eq!(built.document.summary.unowned_source, sources);
    assert_eq!(
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == "scripts/release.sh")
            .expect("shell script")
            .class,
        InventoryClass::Script
    );
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
            &snapshot(root.path()),
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
fn overlapping_generated_declarations_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/client.ts", "generated\n");
    let first = "    - files: [\"generated/**\"]\n      producer: first\n";
    let second = "    - files: [\"generated/client.ts\"]\n      producer: second\n";
    for declarations in [format!("{first}{second}"), format!("{second}{first}")] {
        let error = build(
            root.path(),
            &snapshot(root.path()),
            &manifest(&format!("  generated:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("overlapping producers must fail");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "generated/client.ts" && rules == ["manifest:inventory.generated[0]", "manifest:inventory.generated[1]"])
        );
    }
}

#[test]
fn overlapping_vendored_declarations_are_order_independent_errors() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "vendor/client.ts", "vendored\n");
    let first = "    - files: [\"vendor/**\"]\n      source: first\n      version: '1'\n";
    let second = "    - files: [\"vendor/client.ts\"]\n      source: second\n      version: '2'\n";
    for declarations in [format!("{first}{second}"), format!("{second}{first}")] {
        let error = build(
            root.path(),
            &snapshot(root.path()),
            &manifest(&format!("  vendored:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("overlapping vendors must fail");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "vendor/client.ts" && rules == ["manifest:inventory.vendored[0]", "manifest:inventory.vendored[1]"])
        );
    }
}

#[test]
fn duplicate_absent_generated_declarations_are_errors() {
    let root = tempdir().expect("temporary repository");
    for producers in [["first", "second"], ["second", "first"]] {
        let declarations: String = producers
            .iter()
            .map(|producer| {
                format!("    - files: [\"generated/missing.ts\"]\n      producer: {producer}\n")
            })
            .collect();
        let error = build(
            root.path(),
            &[],
            &manifest(&format!("  generated:\n{declarations}")),
            &BuildConfig::default(),
        )
        .expect_err("an absent path cannot have conflicting producers");
        assert!(
            matches!(error, InventoryError::ClassificationConflict { path, rules }
            if path == "generated/missing.ts" && rules == ["manifest:inventory.generated[0]", "manifest:inventory.generated[1]"])
        );
    }
}

#[test]
fn overlapping_patterns_within_one_generated_declaration_are_not_conflicts() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/client.ts", "generated\n");
    let manifest = manifest(
        "  generated:\n    - files: ['generated/**', 'generated/client.ts', 'absent.ts', 'absent.ts']\n      producer: generate\n",
    );
    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("one declaration owns both paths");
    assert_eq!(built.document.entries.len(), 2);
    assert_eq!(built.document.summary.files, 2);
    assert_eq!(built.document.entries[0].path, "absent.ts");
    assert_eq!(built.document.entries[1].path, "generated/client.ts");
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
        build(
            root.path(),
            &snapshot(root.path()),
            &empty_manifest(),
            &make_config(None)
        ),
        Err(InventoryError::MissingDefaultReplacement {
            default: InventoryClass::Source,
            ..
        })
    ));
    assert!(matches!(
        build(
            root.path(),
            &snapshot(root.path()),
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
        &snapshot(root.path()),
        &empty_manifest(),
        &make_config(Some(InventoryClass::Source)),
    )
    .expect("named replacement is valid");
    assert_eq!(built.document.entries[0].class, InventoryClass::Migration);
}

#[test]
fn manifest_class_override_names_the_replaced_default() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "src/value.ts", "export {};\n");
    let manifest = manifest(
        r#"  classes:
    - class: migration
      files: ["src/value.ts"]
      replaces: source
"#,
    );

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("manifest override should build");
    let entry = &built.document.entries[0];
    assert_eq!(entry.class, InventoryClass::Migration);
    assert_eq!(entry.by, "rule:manifest:inventory.classes[0]");
}

#[test]
fn manifest_class_override_errors_are_order_independent() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "README.md", "docs\n");
    write(root.path(), "src/value.ts", "export {};\n");
    let harmless = r#"    - class: doc
      files: ["README.md"]"#;

    for declarations in [
        format!(
            r#"    - class: migration
      files: ["src/value.ts"]
{harmless}"#
        ),
        format!(
            r#"{harmless}
    - class: migration
      files: ["src/value.ts"]"#
        ),
    ] {
        let manifest = manifest(&format!("  classes:\n{declarations}\n"));
        assert!(matches!(
            build(
                root.path(),
                &snapshot(root.path()),
                &manifest,
                &BuildConfig::default()
            ),
            Err(InventoryError::MissingDefaultReplacement {
                path,
                default: InventoryClass::Source,
                ..
            }) if path == "src/value.ts"
        ));
    }

    for declarations in [
        format!(
            r#"    - class: migration
      files: ["src/value.ts"]
      replaces: test
{harmless}"#
        ),
        format!(
            r#"{harmless}
    - class: migration
      files: ["src/value.ts"]
      replaces: test"#
        ),
    ] {
        let manifest = manifest(&format!("  classes:\n{declarations}\n"));
        assert!(matches!(
            build(
                root.path(),
                &snapshot(root.path()),
                &manifest,
                &BuildConfig::default()
            ),
            Err(InventoryError::WrongDefaultReplacement {
                path,
                expected: InventoryClass::Source,
                named: InventoryClass::Test,
                ..
            }) if path == "src/value.ts"
        ));
    }
}

#[test]
fn generated_provenance_distinguishes_same_producer_inputs() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "generated/first.ts", "generated\n");
    write(root.path(), "generated/second.ts", "generated\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/first.ts"]
      producer: generate
      inputs: ["schema/first.yaml"]
    - files: ["generated/second.ts"]
      producer: generate
      inputs: ["schema/second.yaml"]
"#,
    );

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("generated declarations should build");
    let inputs = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.generated_by.as_ref())
            .map(|generated| generated.inputs.as_slice())
            .expect("generated provenance")
    };
    assert_eq!(inputs("generated/first.ts"), ["schema/first.yaml"]);
    assert_eq!(inputs("generated/second.ts"), ["schema/second.yaml"]);
}

#[test]
fn snapshot_listing_controls_paths_blobs_and_ignored_count() {
    let clean = tempdir().expect("clean checkout");
    let dirty = tempdir().expect("checkout with ignored files");
    write(clean.path(), "src/value.ts", "export {};\n");
    write(clean.path(), "src/z-other.ts", "export {};\n");
    write(dirty.path(), "src/value.ts", "export {};\n");
    write(dirty.path(), "src/z-other.ts", "export {};\n");
    write(dirty.path(), "target/cache.bin", "ignored\n");
    let listing = vec![
        snapshot_entry(
            "src/value.ts",
            InventoryClass::Unknown,
            Some("0123456789012345678901234567890123456789"),
        ),
        snapshot_entry(
            "src/z-other.ts",
            InventoryClass::Unknown,
            Some("0123456789012345678901234567890123456789012345678901234567890123"),
        ),
    ];

    let first = build(
        clean.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("clean inventory");
    let second = build(
        dirty.path(),
        &listing,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("dirty inventory");
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.document.entries, second.document.entries);
    assert_eq!(
        first.document.entries[0].blob.as_deref(),
        Some("sha1:0123456789012345678901234567890123456789")
    );
    assert_eq!(
        first.document.entries[1].blob.as_deref(),
        Some("sha256:0123456789012345678901234567890123456789012345678901234567890123")
    );
    assert_eq!(first.document.summary.ignored_files, Some(0));

    let listing_with_ignored = [
        listing[0].clone(),
        snapshot_entry("target/cache.bin", InventoryClass::Ignored, None),
    ];
    let with_ignored = build(
        dirty.path(),
        &listing_with_ignored,
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("excluded path inventory");
    assert_eq!(with_ignored.document.summary.ignored_files, Some(1));
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

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("inventory builds");
    let unit_of = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .and_then(|entry| entry.unit.as_deref())
    };
    assert_eq!(unit_of("packages/web/src/index.ts"), Some("packages/web"));
    assert_eq!(unit_of("services/api/src/index.ts"), Some("."));
    assert_eq!(unit_of("crates/tool/src/lib.rs"), Some("crates/tool"));
}

#[test]
fn project_references_define_units_and_tsconfig_precedes_deeper_package() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "tsconfig.json",
        r#"{"references":[{"path":"packages/app/tsconfig.build.json"}]}"#,
    );
    write(
        root.path(),
        "packages/app/tsconfig.build.json",
        r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#,
    );
    write(root.path(), "packages/app/src/index.ts", "export {};\n");
    write(
        root.path(),
        "packages/loose/package.json",
        r#"{"name":"loose"}"#,
    );
    write(root.path(), "packages/loose/src/index.ts", "export {};\n");

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("referenced units should build");
    let entry = |path: &str| {
        built
            .document
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .expect("fixture entry")
    };
    assert_eq!(
        entry("packages/app/src/index.ts").unit.as_deref(),
        Some("packages/app")
    );
    assert_eq!(
        entry("packages/loose/src/index.ts").unit.as_deref(),
        Some(".")
    );
    let app_alias = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/app")
        .expect("referenced unit alias row");
    assert_eq!(
        app_alias.alias_table.as_deref(),
        Some("packages/app/tsconfig.build.json")
    );
    assert_eq!(app_alias.by, "tsconfig-reference");
}

#[test]
fn alias_summary_has_one_explicit_row_per_unit() {
    let root = tempdir().expect("temporary repository");
    write(
        root.path(),
        "packages/exported/package.json",
        r#"{"name":"exported","exports":{".":"./src/index.js"}}"#,
    );
    write(
        root.path(),
        "packages/exported/src/index.js",
        "export {};\n",
    );
    write(
        root.path(),
        "packages/plain/package.json",
        r#"{"name":"plain"}"#,
    );
    write(root.path(), "packages/plain/src/index.js", "export {};\n");

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &empty_manifest(),
        &BuildConfig::default(),
    )
    .expect("package units should build");
    assert_eq!(built.units.len(), built.document.summary.unit_aliases.len());
    for unit in &built.units {
        assert_eq!(
            built
                .document
                .summary
                .unit_aliases
                .iter()
                .filter(|row| row.unit == unit.root)
                .count(),
            1,
            "each unit must have exactly one alias row"
        );
    }
    let exported = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/exported")
        .expect("exported unit row");
    assert_eq!(
        exported.alias_table.as_deref(),
        Some("packages/exported/package.json")
    );
    let plain = built
        .document
        .summary
        .unit_aliases
        .iter()
        .find(|row| row.unit == "packages/plain")
        .expect("plain unit row");
    assert_eq!(plain.alias_table, None);
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

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("inventory builds");
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

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
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
fn glob_declared_absent_generated_scope_is_summarized_and_verified() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "source.txt", "input\n");
    let manifest = manifest(
        r#"  generated:
    - files: ["generated/**"]
      producer: "true"
      inputs: ["source.txt"]
      reproducible: true
"#,
    );

    let built = build(
        root.path(),
        &snapshot(root.path()),
        &manifest,
        &BuildConfig::default(),
    )
    .expect("absent generated scope should build");
    assert_eq!(built.document.summary.generated_absent.len(), 1);
    assert_eq!(
        built.document.summary.generated_absent[0].declaration,
        "generated/**"
    );
    assert_eq!(built.document.summary.generated_absent[0].producer, "true");
    assert!(
        built
            .document
            .entries
            .iter()
            .all(|entry| entry.path != "generated/**"),
        "a glob declaration is not a file path"
    );

    let issues = verify_generated(root.path(), &manifest).expect("producer runs");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, GeneratedIssueCode::GeneratedAbsent);
    assert_eq!(issues[0].path, "generated/**");
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
        ..BuildConfig::default()
    };
    let mut listing = snapshot(root.path());
    listing.push(snapshot_entry(
        "external",
        InventoryClass::Submodule,
        Some("sha1:123"),
    ));

    let first = build(root.path(), &listing, &empty_manifest(), &config).expect("first inventory");
    let second =
        build(root.path(), &listing, &empty_manifest(), &config).expect("second inventory");
    let counted: u64 = first.document.summary.by_class.values().sum();
    assert_eq!(counted, first.document.entries.len() as u64);
    assert_eq!(first.document.summary.files, counted);
    assert_eq!(first.document.summary.unowned_source, ["src/unowned.ts"]);
    assert_eq!(first.document.summary.unknown, ["mystery.xyz"]);
    assert_eq!(first.document.summary.submodules[0].path, "external");
    assert_eq!(first.digest, second.digest);
}
