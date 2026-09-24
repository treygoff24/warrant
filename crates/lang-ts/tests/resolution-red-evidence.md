# W1.3 regression red evidence

Observed on 2026-09-24 against `ff85c56`, with one temporary mutation per run.
All 23 mutation runs below exited 101 with the named test failing at an assertion.
Each mutation was restored before applying the next. No mutations are retained.

## Resolution tests

Command for each row: `cargo test --locked -p warrant-lang-ts --test resolution <test> -- --exact`.
Mutations were confined to `crates/lang-ts/src/resolution.rs`.

| Test | Mutation | Observed failure |
| --- | --- | --- |
| `paths_resolve_with_exported_bindings_and_missing_edges_keep_reasons` | Replace alias requests with an absent target. | Alias edge unresolved with `module-not-found`. |
| `conditions_follow_importer_format_and_dynamic_import_syntax` | Force import condition for require importers. | Targets `[5, 5, 5, 5]` instead of `[5, 5, 6, 5]`. |
| `unmatched_export_condition_does_not_fall_back_to_main` | Enable unsupported browser condition. | Unmatched export became resolved. |
| `base_url_lookup_is_removed_from_ts6_but_still_anchors_paths` | Retain baseUrl in TS6+. | TS `6.0.0` bare import resolved; expected false. |
| `project_references_select_the_referenced_config_not_the_solution` | Disable project references. | Target `None` instead of `Some(4)`; `module-not-found`. |
| `node_builtins_are_external_and_missing_packages_are_unresolved` | Disable builtin recognition. | External target `None` instead of `node:fs`. |
| `star_reexports_bind_consumers_and_materialize_module_cycles` | Skip symbol binding materialization. | Assertion `star export binding is observed` failed. |
| `conflicting_star_exports_do_not_invent_a_binding` | Accept ambiguous star origins. | Ambiguous import unexpectedly had a target symbol. |
| `captured_dependency_packages_are_external_but_uncaptured_packages_are_not_guessed` | Lose external package identity. | External target `None` instead of `dep`. |
| `inherited_paths_use_the_defining_config_directory` | Lose inherited path mappings. | Inherited alias target `None` instead of `Some(4)`. |
| `invalid_configs_preserve_each_unresolved_edge` | Lose config error reasons. | Per-edge `resolution-failed:` reason predicate failed. |
| `namespace_imports_observe_all_exported_bindings` | Skip namespace expansion. | Exported names empty instead of `{other, value}`. |
| `exported_import_bindings_keep_the_original_symbol_consumer_chain` | Skip exported import reexport links. | Consumer-chain count 0 instead of 1. |

## Range regression before the fix

Command: `cargo test --locked -p warrant-lang-ts --test resolution ts5_ranges_preserve_base_url_lookup`.
Run against `17da687` with only the two new tests added. Exit 101: both
`declared_ts5_ranges_preserve_base_url_lookup` and
`installed_ts5_ranges_preserve_base_url_lookup` failed for `>=5.0.0`.
Both observed an unresolved bare import with `module-not-found` and target `None`
instead of `Some(4)`. After the prefix fix, all 15 resolution tests passed.

## CLI assertions

Command for each row: `cargo test --locked -p warrant-cli --test conformance -- <test> --exact`.
Capability mutations changed `crates/lang-ts/src/lib.rs`; other mutations changed
`crates/cli/src/commands/model.rs`. Table mutations cleared only the named query
result rows before output, leaving edges and other table results intact.

| Test | Mutation | Observed failure |
| --- | --- | --- |
| `model_capabilities_remain_unqualified` | Claim parity without qualification. | Authority `parity-qualified` instead of `unqualified`. |
| `model_capabilities_remain_unqualified` | Claim a qualified mode. | Qualified modes `[nodenext]` instead of `[]`. |
| `model_capabilities_remain_unqualified` | Omit project references capability. | Required capability missing from `supports`. |
| `model_conformance` | Corrupt persisted edge target projection. | `baseurl-ts6`: inconsistent unresolved edge with `wrong-target`. |
| `model_conformance` | Lose edge target binding projection. | `cycle`: target symbols null instead of `leaf` and `value`. |
| `model_conformance` | Drop analysis_inputs output rows. | `baseurl-ts6`: `analysis_inputs` differs (empty rows). |
| `model_conformance` | Drop module_edges output rows. | `cycle`: `module_edges` differs (empty rows). |
| `model_conformance` | Drop module_cycles output rows. | `cycle`: `module_cycles` differs (empty rows). |
| `model_conformance` | Drop symbol_consumers output rows. | `reexport-chain`: `symbol_consumers` differs (empty rows). |
| `model_conformance` | Drop observed_interfaces output rows. | `cycle`: `observed_interfaces` differs (empty rows). |

## Restored verification

- `cargo test --locked -p warrant-lang-ts`: exit 0; 8 analysis tests and 15 resolution tests passed.
- `cargo test --locked -p warrant-cli --test conformance -- model`: exit 0; all 4 matching tests passed.
