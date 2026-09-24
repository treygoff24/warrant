# Warrant program model

`warrant query --schema` exposes the SQLite model described here. One immutable
database is written for each source snapshot. IDs are stable integration-provided
integers; spans are byte offsets. Boolean columns use SQLite `0` and `1`.

## Tables

| Name | Columns | Purpose |
| --- | --- | --- |
| `meta` | `key`, `value` | Snapshot manifest, tool and integration identities, and `model_digest`. |
| `units` | `id`, `integration`, `root`, `config_path`, `kind` | Compilation and package scopes. |
| `files` | `id`, `path`, `blob`, `class`, `language`, `unit_id`, `module_id` | Inventoried files connected to units and optional declared modules. |
| `modules` | `id`, `name`, `declared_by`, `intent`, `owner` | Declared architectural modules. |
| `symbols` | `id`, `file_id`, `name`, `kind`, `exported`, `export_name`, `visibility`, `span_start`, `span_end` | Per-file symbol bindings. |
| `edges` | `id`, `kind`, `from_file`, `from_symbol`, `to_file`, `to_symbol`, `to_external`, `resolved`, `unresolved_reason`, `type_only`, `span_start`, `span_end`, `basis` | Imports, re-exports, dynamic imports, requires, registrations, declarations, and configuration references. |
| `references` | `id`, `file_id`, `symbol_id`, `span_start`, `span_end` | In-file uses of imported bindings. |
| `entrypoints` | `id`, `file_id`, `symbol_id`, `kind`, `basis`, `declared_by` | Runtime, framework, test, script, migration, registry, and declared entrypoints. |
| `effects` | `id`, `name`, `symbol_id`, `declared_by` | Effects attached to symbols by declarations. |
| `unsupported` | `id`, `file_id`, `construct`, `span_start`, `span_end`, `reason` | Constructs an enabled integration could not analyze. |
| `capability_reports` | `integration`, `json` | Canonical JSON capability report for each integration. |

## Materialized views

SQLite has no native materialized-view object, so these are tables rebuilt once
when model writing finishes.

| Name | Columns | Purpose |
| --- | --- | --- |
| `module_edges` | `from_module`, `to_module`, `edge_count`, `type_only_count`, `first_edge_id` | Aggregated cross-module file edges. |
| `symbol_consumers` | `symbol_id`, `consumer_file`, `via_reexport_chain` | Files consuming a symbol and whether the observed route includes a re-export. |
| `module_cycles` | `cycle_id`, `module_id`, `position` | Deterministically ordered strongly connected module components. |

`unowned_files` is a normal SQLite view over source-class `files` whose
`module_id` is null.

## Query limits and safety

SQL queries open the database with SQLite's read-only flag and an authorizer that
allows only select, read, function, and recursive-select actions. Statements such
as `INSERT`, `UPDATE`, `DELETE`, `ATTACH`, and write pragmas are rejected. Results
are bounded by row and encoded-row byte limits. A bounded result sets
`truncated: true` and includes an `OFFSET` continuation hint.
If the first row alone exceeds the byte cap, the query returns a `row-too-large`
error identifying row 1, its encoded size, and the cap instead of a continuation
that cannot advance. Increase the cap or select fewer or narrower columns.

The model digest is SHA-256 over a canonical, fixed-table-order dump. Rows are
sorted by stable keys, `model_digest` is excluded from its own input, and
incidental timestamp metadata is omitted.
