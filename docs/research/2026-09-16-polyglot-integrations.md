# Polyglot language integrations for Warrant

Status: research report, 2026-09-16. Author: Fable (research lane), for Warrant. Versions verified live on 2026-09-16.

## Purpose and framing

Warrant's core is language-neutral: a snapshot, an inventory, a program model in SQLite, contracts evaluated against that model, and a capability report per language integration. TypeScript/JavaScript is the first integration. This report surveys what a Rust, Python, Go, SQL, interface-document, and deployment-configuration integration would lean on, so the spec can define the integration contract once and have the TypeScript integration prove it.

Two framing decisions run through every section. First, an integration's job is to produce evidence with a stated authority, not to be complete: the vision draft says "A file that couldn't be read or a construct that couldn't be analyzed appears as missing evidence, not as silence" (docs/design/2026-09-16-specgate-vision.md), and W04 asks for "language integrations with published capability and completeness contracts." Second, wherever a real compiler or build tool already resolves the program, Warrant reads that resolution rather than reimplementing it, which is what W02 means by "the same program the build uses."

Every version below was read from a registry or repository on 2026-09-16. Where a claim could not be verified from a primary source this session it is marked unverified.

## 1. Rust

### Inventory source: cargo metadata

`cargo metadata` is the right inventory source for a Rust workspace. The Cargo book documents the JSON output and recommends passing `--format-version 1` to future-proof consumers (https://doc.rust-lang.org/cargo/commands/cargo-metadata.html, as of 2026-09-16). The output carries the workspace member list (`workspace_members`, `workspace_default_members`), each package's targets with a `kind` array whose values include `lib`, `bin`, `test`, `bench`, `example`, `custom-build` for build scripts, and `proc-macro`, each package's `features` table, and a `resolve` graph of the resolved dependency set. The `cargo_metadata` crate deserializes this into typed structs (0.23.1, 2025-11-12, MIT, https://github.com/oli-obk/cargo_metadata, as of 2026-09-16).

What this gives the inventory for free: crate boundaries, target roots (`src/main.rs`, `src/lib.rs`, `build.rs`, `tests/*.rs`, `examples/*.rs`), the feature matrix, and the external dependency set with versions. What it does not give: the module tree inside a crate, and which `cfg` combinations actually compile. Cargo metadata resolves dependencies for all targets unless filtered, so the inventory should record the feature set and target filter it was taken with.

Stable Rust was 1.98.1 on 2026-09-16 (https://static.rust-lang.org/dist/channel-rust-stable.toml, as of 2026-09-16).

### Module tree and use graph: four options

| Option | Version (as of 2026-09-16) | License | Source |
| --- | --- | --- | --- |
| syn walking | syn 3.0.6, 2026-09-16 (3.0.0 released 2026-07-18) | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| rust-analyzer as a library (`ra_ap_*`) | ra_ap_syntax, ra_ap_hir, ra_ap_ide 0.0.352, 2026-09-14 | MIT OR Apache-2.0 | https://github.com/rust-lang/rust-analyzer |
| rust-analyzer as an LSP or CLI | release 2026-09-14 | Apache-2.0 (repository field) | https://github.com/rust-lang/rust-analyzer/releases |
| rustdoc JSON | rustdoc-types 0.61.0, 2026-07-29, FORMAT_VERSION 61 | MIT OR Apache-2.0 | https://github.com/rust-lang/rustdoc-types |

syn-based walking. syn parses a file into an AST without resolving names. A walker can collect `mod` declarations (inline and file-backed), `use` trees, and item visibility, and it can follow `#[path]` attributes and `#[cfg]` attributes syntactically. It cannot resolve what a `use` path actually names once glob imports, re-exports, macros, or `cfg`-dependent modules enter the picture, and it never sees macro-generated items. syn 3 shipped on 2026-07-18 and is at 3.0.6 (https://github.com/dtolnay/syn/releases, as of 2026-09-16); much of the ecosystem is still on syn 2, so a Warrant integration should pin one major and record it.

rust-analyzer as a library. rust-analyzer publishes its internal crates to crates.io under the `ra_ap_` prefix from an autopublish workflow that runs on pushes to the release branch (https://github.com/rust-lang/rust-analyzer/blob/master/.github/workflows/autopublish.yaml, as of 2026-09-16). The current version is 0.0.352 (crates.io, 2026-09-14). The project's architecture document states the API position plainly: "The `ide` API of rust-analyzer are explicitly unstable, but the LSP interface is stable" (https://github.com/rust-lang/rust-analyzer/blob/master/docs/book/src/contributing/architecture.md, as of 2026-09-16). The practical consequence shows in cargo-modules, which pins `ra_ap_* = "=0.0.345"` while 0.0.352 is current (https://github.com/regexident/cargo-modules/blob/master/Cargo.toml, as of 2026-09-16). Linking these crates gives the best possible Rust semantic model, including name resolution through re-exports and macro expansion, at the cost of a large dependency tree and a version pin that must move deliberately.

rust-analyzer as a process. The binary exposes both the LSP and a batch CLI. Its command-line flags file defines `analysis-stats`, `lsif`, and `scip` subcommands; `scip` takes a project path, an optional `--output` path defaulting to `index.scip`, a `--config-path`, an `--exclude-vendored-libraries` flag, and a worker-thread count for cache priming (https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/cli/flags.rs, as of 2026-09-16). Running `rust-analyzer scip` and reading the index through the `scip` crate gives Warrant a definitions-and-references graph without linking any `ra_ap_` crate. The LSP route is the same engine with worse batch ergonomics: a long-lived server, cache priming, and one round trip per symbol.

rustdoc JSON. The rustdoc book lists `--output-format json` under unstable features and requires nightly with `-Z unstable-options` (https://doc.rust-lang.org/nightly/rustdoc/unstable-features.html, as of 2026-09-16). The tracking issue rust-lang/rust#76578 is still open (https://github.com/rust-lang/rust/issues/76578, last updated 2025-10-25, as of 2026-09-16), and Cargo's `--output-format` flag for `cargo rustdoc` is tracked separately as unstable under issue 13283 (https://doc.rust-lang.org/cargo/reference/unstable.html, as of 2026-09-16). The `rustdoc-types` crate (0.61.0) carries the type definitions and its README says it deserializes "the output of `cargo +nightly rustdoc -- --output-format json -Z unstable-options`" (https://github.com/rust-lang/rustdoc-types, as of 2026-09-16). rustdoc JSON is the cleanest public-interface source, because it is exactly the set of reachable items rustdoc documents, but it needs nightly and a format that has moved 61 times. It fits an opt-in public-interface evidence kind, not the default path.

### Visibility as the interface signal

The Rust reference defines `pub`, `pub(crate)`, `pub(super)`, and `pub(in path)` (https://doc.rust-lang.org/reference/visibility-and-privacy.html, as of 2026-09-16). Syntactic visibility is a real interface signal with one large caveat: an item that is `pub` inside a private module is not reachable from outside the crate unless re-exported, and reachability requires resolution. A syn walker can record declared visibility; only rust-analyzer or rustdoc can report effective reachability. The capability report should name which one it is reporting.

### Prior art

cargo-modules (0.27.0, 2026-08-03, MPL-2.0, https://github.com/regexident/cargo-modules, as of 2026-09-16) offers `structure` (module tree), `dependencies` (internal dependency graph), and `orphans` (unlinked source files), built on the `ra_ap_` crates. It is the closest existing tool to a Rust module-boundary checker, and its MPL-2.0 license permits use as a separate process or as a file-level copyleft dependency, but not silent vendoring into an MIT-licensed core. cargo-deny (0.20.2, 2026-07-09, MIT OR Apache-2.0, https://github.com/EmbarkStudios/cargo-deny, as of 2026-09-16) checks licenses, bans, advisories, and sources over the cargo metadata graph. It is prior art for a contract that scopes an external dependency set, and its `deny.toml` is the kind of declared policy Warrant would rather consume than reinvent.

### What Rust entrypoints look like

Cargo metadata names most of them: every `bin` target has a `main`, every `custom-build` target is a `build.rs` that runs at build time, every `proc-macro` target exports macros the compiler invokes. Beyond that, entrypoints are attribute-driven: `#[test]` functions, `#[tokio::main]` (https://docs.rs/tokio/latest/tokio/attr.main.html, as of 2026-09-16), clap's derive-based `Subcommand` enums (https://docs.rs/clap/latest/clap/_derive/index.html, as of 2026-09-16), `#[proc_macro]` and its derive and attribute forms (https://doc.rust-lang.org/reference/procedural-macros.html, as of 2026-09-16), and build scripts (https://doc.rust-lang.org/cargo/reference/build-scripts.html, as of 2026-09-16). Registration-style entrypoints (distributed slices, `inventory`-style collection, `#[no_mangle]` exports for FFI) need declared evidence paths, exactly as W02 says for dynamic registration in any language.

### Smallest credible v1 Rust integration

The goal is for Warrant to check its own module boundaries and dependency direction. The smallest integration that does that honestly:

1. Inventory from `cargo metadata --format-version 1` via the `cargo_metadata` crate: workspace members, targets and kinds, features, resolved external dependencies. Record the feature set and the cargo version in the receipt.
2. Module tree and `use` edges from a syn walker over each target root, following `mod` declarations and `#[path]`, recording declared visibility on every item, and recording `#[cfg]` attributes as edge qualifiers rather than evaluating them.
3. Edges classified as `use` (syntactic), `mod` (containment), and `extern-crate` (from the `resolve` graph), with every `use` edge marked resolution: syntactic. Glob imports, macro-generated items, and re-export chains are reported as unresolved forms in the capability report.
4. Optional second pass: `rust-analyzer scip` when the binary is present and pinned, upgrading `use` edges to resolution: compiler where the SCIP index confirms the definition, and adding reference edges. Absent or unpinned rust-analyzer is reported as a missing instrument, not silently skipped.

Module ownership and dependency-direction contracts need only steps 1 through 3, because those contracts are about which module declares a dependency on which, and a syntactic `use` is a declared dependency. Symbol-level reference contracts and dead-code claims need step 4. The capability report says which steps ran.

## 2. Python

Python has no compiler resolution to borrow, so every option is either an import-graph builder that resolves module names by package layout or a type checker that resolves symbols by inference.

| Tool | Version (as of 2026-09-16) | License | Source |
| --- | --- | --- | --- |
| grimp | 3.17, 2026-09-04 | BSD-2-Clause | https://github.com/seddonym/grimp |
| import-linter | 2.15, 2026-09-04 | BSD-2-Clause | https://github.com/seddonym/import-linter |
| pydeps | 3.0.8, 2026-09-07 | BSD-2-Clause | https://github.com/thebjorn/pydeps |
| ruff | 0.16.8, 2026-09-16 | MIT | https://github.com/astral-sh/ruff |
| ruff_python_parser (crate) | 0.0.14, 2026-09-16 | MIT | https://crates.io/crates/ruff_python_parser |
| ty | 0.0.81, 2026-09-15 | MIT | https://github.com/astral-sh/ty |
| pyright (npm) | 1.1.414, 2026-09-09 | MIT | https://github.com/microsoft/pyright |
| basedpyright (npm) | 1.40.1, 2026-09-10 | MIT | https://github.com/DetachHead/basedpyright |
| @sourcegraph/scip-python | 0.6.6, 2025-09-05 | MIT (npm manifest) | https://github.com/sourcegraph/scip-python |

### Import graph tools

grimp "Builds a queryable graph of the imports within one or more Python packages" and exposes children, descendants, direct imports, and path queries as a Python API (https://github.com/seddonym/grimp/blob/main/README.rst, as of 2026-09-16). Its changelog records that the graph was reimplemented in Rust and that import parsing moved from Python's `ast` module to Rust (https://github.com/seddonym/grimp/blob/main/CHANGELOG.rst, as of 2026-09-16), so it is fast enough to run on every snapshot. import-linter sits on grimp and evaluates contract types documented as `forbidden`, `independence`, `layers`, `protected`, and `acyclic_siblings` (https://github.com/seddonym/import-linter/tree/master/docs/contract_types, as of 2026-09-16). Those five are the closest published vocabulary to Warrant's dependency-direction contracts, and a Warrant Python integration should be able to express each of them. pydeps draws the graph over the standard library's ModuleFinder machinery (its README exposes `--debug-mf` for "the ModuleFinder.debug flag," https://github.com/thebjorn/pydeps/blob/master/README.rst, as of 2026-09-16); it is a visualization tool, not an evidence source.

ruff ships `ruff analyze graph`, whose source describes it as "Generate a map of Python file dependencies or dependents" with a `--direction` option and JSON output (https://github.com/astral-sh/ruff/blob/main/crates/ruff/src/args.rs, as of 2026-09-16). The command checks the `analyze.preview` setting and refuses when preview is disabled (https://github.com/astral-sh/ruff/blob/main/crates/ruff/src/commands/analyze_graph.rs, as of 2026-09-16), so it is a preview feature today. It is attractive because ruff is already installed almost everywhere and emits a file-to-files map that maps directly onto Warrant's import edges.

### ruff's parser as a library

`ruff_python_parser` is on crates.io at 0.0.14 and its README states: "This crate is an internal component of Ruff. The Rust API exposed here is unstable and will have frequent breaking changes" (https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_parser/README.md, as of 2026-09-16). Ruff's versioning policy confirms that only `ruff`, `ruff_linter`, and `ruff_wasm` follow the normal policy, and "The remaining crates published as part of Ruff releases provide no stability guarantees" and "are versioned as 0.0.x" with the patch bumped on every release (https://docs.astral.sh/ruff/versioning/, as of 2026-09-16). The same policy covers `ty_python_semantic` (0.0.14 on crates.io, as of 2026-09-16), the semantic layer behind Astral's `ty` type checker (https://github.com/astral-sh/ty, 0.0.81 on PyPI, as of 2026-09-16). Linking these crates gives a native Rust Python parser with the same pin-and-move discipline the `ra_ap_` crates need. The parser alone resolves nothing; module resolution against `sys.path`, namespace packages, and `__init__` re-exports is the part that matters and lives in ty or pyright.

### pyright and basedpyright as reference sources

pyright's command line supports `--outputjson` for diagnostics, `--verifytypes`, and `--createstub`, but no batch references command (https://github.com/microsoft/pyright/blob/main/docs/command-line.md, as of 2026-09-16). References come only through the LSP, one `textDocument/references` request per symbol against a long-lived server. basedpyright is a fork with the same license and a broader CLI (https://docs.basedpyright.com/latest/, as of 2026-09-16). scip-python is Sourcegraph's SCIP indexer built on pyright; its npm release is 0.6.6 from 2025-09-05 (https://www.npmjs.com/package/@sourcegraph/scip-python, as of 2026-09-16), a year old against a pyright that releases weekly, which is the churn risk in concrete form. For batch reference extraction, scip-python is still the only option that writes a file instead of answering requests.

### Entrypoints

Python entrypoints are declared in packaging metadata and in framework decorators. The packaging specifications define entry point groups, with `console_scripts` and `gui_scripts` as the installer-recognized groups (https://packaging.python.org/en/latest/specifications/entry-points/, as of 2026-09-16), and the pyproject specification maps them to `[project.scripts]`, `[project.gui-scripts]`, and `[project.entry-points]` tables (https://packaging.python.org/en/latest/specifications/pyproject-toml/, as of 2026-09-16). `__main__.py` makes a package runnable. pytest's discovery searches for `test_*.py` or `*_test.py` files (https://docs.pytest.org/en/stable/explanation/goodpractices.html, as of 2026-09-16). Django routes are declared in URL configuration modules (https://docs.djangoproject.com/en/6.1/topics/http/urls/, as of 2026-09-16), FastAPI registers handlers through decorators such as `@app.get` (https://fastapi.tiangolo.com/tutorial/first-steps/, as of 2026-09-16), and Celery registers tasks through `@app.task` and `shared_task` (https://docs.celeryq.dev/en/stable/userguide/tasks.html, as of 2026-09-16). All of the decorator forms are pattern checks: the integration can see that a function is decorated with something named `app.get`, and the capability report must say it did not verify that `app` is a FastAPI application.

### Recommended Python shape

Inventory from pyproject plus the filesystem. Import edges from `ruff analyze graph` where preview is acceptable, or from grimp as a subprocess, both marked as resolved by package layout rather than by an interpreter. Reference edges from scip-python when present, marked as inference. Contracts express the five import-linter types. Everything decorator-based is an entrypoint candidate with pattern-check authority.

## 3. Go

Go is the easiest polyglot target because the toolchain exposes its own package model in JSON and the analysis libraries are first-party.

| Tool | Version (as of 2026-09-16) | License | Source |
| --- | --- | --- | --- |
| Go | go1.27.1 | BSD-3-Clause | https://go.dev/dl/ |
| golang.org/x/tools | v0.50.0, 2026-09-08 | BSD-3-Clause | https://pkg.go.dev/golang.org/x/tools |
| gopls | v0.23.0, released 2026-07-09 | BSD-3-Clause | https://github.com/golang/tools/releases/tag/gopls/v0.23.0 |
| scip-go | v0.2.7, 2026-05-25 | Apache-2.0 | https://github.com/sourcegraph/scip-go |

`go list -json` prints a struct per package with `ImportPath`, `Module`, `GoFiles`, `TestGoFiles`, `XTestGoFiles`, `EmbedFiles`, `Imports`, and `Deps` among its fields (https://pkg.go.dev/cmd/go#hdr-List_packages_or_modules, as of 2026-09-16). That is an inventory and an import graph in one command, resolved by the same tool that builds the program, with build tags and module boundaries applied. For deeper work, `golang.org/x/tools/go/packages` loads packages with a `LoadMode` bitmask (`NeedFiles`, `NeedSyntax`, `NeedTypes`, `NeedImports`, `NeedDeps`) (https://pkg.go.dev/golang.org/x/tools/go/packages, as of 2026-09-16), and `golang.org/x/tools/go/callgraph` provides `cha`, `rta`, `static`, and `vta` call-graph algorithms with a `cmd/callgraph` driver (https://pkg.go.dev/golang.org/x/tools/go/callgraph and https://github.com/golang/tools/tree/master/cmd, as of 2026-09-16). `guru` is not present in the x/tools `cmd` directory as of 2026-09-16; treat it as gone.

gopls exposes references from the command line, but its documentation says the interface "is currently experimental and subject to change at any point" and "Its primary use is as a debugging aid," pointing to go.dev/issue/63693 for its future (https://github.com/golang/tools/blob/master/gopls/doc/command-line.md, as of 2026-09-16). For batch references, scip-go writes a SCIP index and is the stable path.

Go entrypoints: `func main` in `package main` (a `bin` in cargo terms), `func init` in any package (runs on import), `TestXxx`, `BenchmarkXxx`, and `FuzzXxx` functions in `_test.go` files, `//go:generate` directives, `//go:embed` patterns (visible in `EmbedFiles`), and registration calls such as `http.HandleFunc` or a router's method chain. The first four are discoverable from `go list` output and file naming; handler registration is a pattern check.

The Rust core will drive Go through subprocesses and JSON, and the capability report should record the `go version` string, `GOFLAGS`, and build tags in effect, since all three change what `go list` returns.

## 4. SQL and data

### Migrations as first-class inventory items

A migration is a source file whose effect is a schema change, and the inventory should classify it as such with its producer. The producers seen most often each leave a recognizable footprint: Alembic revision scripts (alembic 1.20.0, 2026-09-11, MIT, https://pypi.org/project/alembic/, as of 2026-09-16), Django migration modules, `drizzle-kit generate`, which writes `.sql` migration files alongside `snapshot.json` state files (drizzle-kit 0.31.10, 2026-03-17, MIT, https://orm.drizzle.team/docs/drizzle-kit-generate, as of 2026-09-16), Prisma's schema file and migration directories (https://www.prisma.io/docs/orm/prisma-schema/overview, as of 2026-09-16; the npm `latest` tag for `prisma` pointed at 8.0.0-rc.15 on 2026-09-16, a release candidate, https://www.npmjs.com/package/prisma), and sqlc's configuration, whose `sql` block names `schema` and `queries` paths and an `engine`, with a `gen` block for output (sqlc v1.31.1, 2026-04-22, MIT, https://docs.sqlc.dev/en/latest/reference/config.html, as of 2026-09-16). An inventory rule per producer is a declared classification whose reason names the producer.

### Schema extraction

sqlparser (0.63.0, 2026-09-13, Apache-2.0, https://github.com/apache/datafusion-sqlparser-rs, as of 2026-09-16) is the Rust SQL parser and now lives under the Apache DataFusion project. Its README is explicit about its limits: "This crate provides only a syntax parser, and tries to avoid applying any SQL semantics, and accepts queries that specific databases would reject, even when using that Database's specific `Dialect`" (https://github.com/apache/datafusion-sqlparser-rs/blob/main/README.md, as of 2026-09-16). For Warrant that is the right tool for reading `CREATE TABLE`, `ALTER TABLE ... ADD CONSTRAINT`, and `CREATE INDEX` statements out of migration files and recording declared tables, columns, and constraints as syntactic facts. It cannot tell you the schema that results from applying twenty migrations in order, because that requires semantics (drops, renames, dialect-specific defaults).

The database catalog is the authoritative schema. PostgreSQL 18 is the current major (18.6 latest minor, https://www.postgresql.org/versions.json, as of 2026-09-16), and `pg_constraint` records each constraint's type in `contype`, its table in `conrelid`, its columns in `conkey`, referenced columns in `confkey`, and check expressions in `conbin` (https://www.postgresql.org/docs/current/catalog-pg-constraint.html, as of 2026-09-16); `information_schema.table_constraints` is the portable view (https://www.postgresql.org/docs/current/infoschema-table-constraints.html, as of 2026-09-16). A catalog query needs a live database at a known migration state, which makes it an observation evidence kind rather than a static one, and the receipt must record which database and which migration head it observed.

Schema-as-code files (drizzle's TypeScript schema, Prisma's `.prisma` file, sqlc's `schema.sql`) sit between the two: declarations the producer turns into migrations, parseable statically. Drizzle's schema is TypeScript, so the TypeScript integration already sees it as symbols; a SQL integration adds the interpretation "this exported constant declares table `users`."

### Linking a database constraint to an application invariant

W04 asks that "A database constraint or generated schema that supports an invariant should appear beside the application code relying on it." Two mechanisms are possible.

Declared links: a contract says "invariant `one-active-subscription-per-account` is enforced by unique index `subscriptions_account_active_idx` and by module `billing/subscriptions`," and Warrant verifies that both named things exist in the snapshot (the index in a migration or catalog, the module in the program model) and reports if either disappears. This is cheap, honest, and matches the vision's rule that contracts carry "how it is enforced, and what that enforcement cannot see."

Inference: follow the ORM call (`db.insert(subscriptions)`) to the table symbol to the constraint. This requires modeling the ORM's query builder, a per-ORM integration of its own, worth doing for one ORM later and never as the v1 mechanism.

### What a state-owner contract would need

A "module X owns table T" contract needs three things the model does not have yet: table symbols (from sqlparser over migrations or from a schema file), write-effect edges from code to tables (the ORM call or raw query site, an effect evidence kind under W07), and a reader-versus-writer distinction on those edges. With those, the contract "only `billing/subscriptions` writes `subscriptions`" is an ordinary edge-scoped rule. The honest capability report says that raw SQL strings built at runtime are unresolved, and that a write through a stored procedure or trigger is invisible to static analysis.

## 5. Interface definitions across languages

| Standard or tool | Version (as of 2026-09-16) | License | Source |
| --- | --- | --- | --- |
| OpenAPI Specification | 3.2.1, released 2026-09-10 | Apache-2.0 (repository) | https://spec.openapis.org/oas/latest.html |
| protobuf tooling: buf | v1.73.0, 2026-09-11 | Apache-2.0 | https://github.com/bufbuild/buf |
| protobuf in Rust: prost, prost-build | 0.14.4, 2026-06-07 | Apache-2.0 | https://github.com/tokio-rs/prost |
| GraphQL specification | September 2025 release | OWFa 1.0 (unverified this session) | https://spec.graphql.org/ |
| JSON Schema | 2020-12 is the latest meta-schema | BSD-style (unverified this session) | https://json-schema.org/specification |
| Model Context Protocol | current protocol version 2026-07-28 | MIT (unverified this session) | https://modelcontextprotocol.io/specification/versioning |
| AsyncAPI | 3.1.0, tagged 2026-01-31 | Apache-2.0 | https://github.com/asyncapi/spec |

Each of these is a document that declares an interface independently of any implementing language. That is exactly what Warrant needs for cross-language edges: the document is a node in the model with its own symbol namespace (operations, messages, tools, types), and each language integration contributes edges from code to the document (server implements operation, client calls operation, generated file was produced from document).

OpenAPI 3.2.1 is current (https://spec.openapis.org/oas/latest.html, title "OpenAPI Specification v3.2.1", as of 2026-09-16). Protobuf has no single spec version worth pinning; buf is the toolchain that lints, breaks-checks, and generates from a `buf.gen.yaml` (https://buf.build/docs/configuration/v2/buf-gen-yaml/, as of 2026-09-16), and prost-build generates Rust. GraphQL's latest release is September 2025 (https://spec.graphql.org/, as of 2026-09-16). JSON Schema's site says the "latest meta-schema is 2020-12" (https://json-schema.org/specification, as of 2026-09-16). MCP's versioning page says "The current protocol version is 2026-07-28," and that version's tools page requires servers with the tools capability to answer `tools/list` with tool definitions carrying an `inputSchema` (https://modelcontextprotocol.io/specification/2026-07-28/server/tools, as of 2026-09-16). AsyncAPI is at 3.1.0 with a JavaScript parser at 3.6.3 (https://www.npmjs.com/package/@asyncapi/parser, as of 2026-09-16).

### The rename scenario

W04's example: "A public command renamed in one language should lead me to affected adapters and consumers elsewhere." With declared interface documents this works in four steps, none of which needs cross-language symbol resolution.

1. The interface document is inventoried and parsed into interface symbols: operation `debrief.undo`, its request and response shapes.
2. Each implementation is linked to the document by a registration edge: the TypeScript handler registered for `debrief.undo` (seen through the registration call), the Rust adapter matching the same string literal (a weak but honest syntactic edge), the MCP tool registry entry with that name.
3. Generated clients carry provenance: the generator, its version, the source document digest, and the generation configuration. openapi-typescript (7.13.0, MIT), orval (8.33.0, MIT), openapi-generator-cli (2.41.0, Apache-2.0), graphql-codegen (7.4.1, MIT), buf generate, and prost-build all fit (npm registry and https://buf.build/docs/generate/overview/, as of 2026-09-16). openapi-generator documents a `.openapi-generator-ignore` file in its output (https://openapi-generator.tech/docs/customization/, as of 2026-09-16); whether it also writes a file manifest and version marker was not verified this session.
4. A rename in the document changes an interface symbol; every edge to the old symbol becomes dangling, and Warrant lists their sources: the handler, the match arm, the tool entry, and every generated client whose recorded source digest no longer matches.

The reproducibility check is a re-run: Warrant invokes the recorded generator at the recorded version on the recorded document and compares the output digest to the snapshot. A mismatch is a finding, and an absent generator is missing evidence. This is W01's "Generated code needs a producer and reproducibility check, not just a convenient exemption," made concrete.

## 6. Deployment configuration

Deployment files are where entrypoints and effects hide from every language integration. The question per file type is whether it names an entrypoint, an effect, or only a build recipe.

| File type | Carries | Reference (as of 2026-09-16) |
| --- | --- | --- |
| Dockerfile | entrypoint (`ENTRYPOINT`, `CMD`), effect-adjacent (`HEALTHCHECK`, `ONBUILD`) | https://docs.docker.com/reference/dockerfile/ |
| Compose | entrypoint per service (`command`, `entrypoint`), topology (`depends_on`, `build`, `healthcheck`) | https://github.com/compose-spec/compose-spec/blob/main/05-services.md |
| GitHub Actions | triggers (`on`, `workflow_dispatch`, `workflow_call`), steps (`run`, `uses`) | https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax |
| systemd unit | entrypoint (`ExecStart` and siblings) | https://man7.org/linux/man-pages/man5/systemd.service.5.html |
| Vercel `vercel.json` | entrypoints (`crons`, `functions`), build (`buildCommand`) | https://vercel.com/docs/project-configuration |
| Netlify `netlify.toml` | build (`[build]`), entrypoints (`[functions]`), routing (`[[redirects]]`, `[[headers]]`, `[context]`) | https://docs.netlify.com/configure-builds/file-based-configuration/ |
| Terraform | effects (every `resource` block) | https://developer.hashicorp.com/terraform/language/resources |

Dockerfile `CMD` and `ENTRYPOINT`, compose `command`, systemd `ExecStart`, Vercel `crons`, and Actions `run` steps all name a command line, and that command line usually names a binary or script the inventory already classified. The edge "deployment file X invokes entrypoint Y" is what makes an otherwise-unreferenced `main` reachable, which is why W01 wants deployment entrypoints in the inventory.

Terraform is the odd one out: every resource block is an effect with no code entrypoint. Terraform itself moved to the Business Source License (https://github.com/hashicorp/terraform/blob/main/LICENSE, as of 2026-09-16), while OpenTofu (v1.12.6, MPL-2.0, https://github.com/opentofu/opentofu, as of 2026-09-16) keeps the HCL dialect under an open license. Warrant only needs to parse HCL, for which tree-sitter-hcl exists (see section 8), and never needs to link either tool.

### Analyzers that emit SARIF

| Analyzer | Version (as of 2026-09-16) | License | SARIF | Source |
| --- | --- | --- | --- | --- |
| hadolint | v2.15.1, 2026-07-31 | GPL-3.0 | `--format sarif` | https://github.com/hadolint/hadolint |
| actionlint | v1.7.12, 2026-03-30 | MIT | via `-format` template shipped in testdata | https://github.com/rhysd/actionlint/blob/main/docs/usage.md |
| tfsec | v1.28.14, 2025-05-02 | MIT | yes | https://github.com/aquasecurity/tfsec |
| trivy | v0.74.0, 2026-08-14 | Apache-2.0 | yes | https://github.com/aquasecurity/trivy |
| checkov | 3.3.17, 2026-09-10 | Apache-2.0 | `-o sarif` | https://github.com/bridgecrewio/checkov |

hadolint's README lists `sarif` among its output formats (https://github.com/hadolint/hadolint/blob/master/README.md, as of 2026-09-16). Its GPL-3.0 license is fine for Warrant because Warrant runs it as a separate process and reads its output; Warrant must never link or vendor it. actionlint documents SARIF through its `-format` template mechanism with a template in its test data (https://github.com/rhysd/actionlint/blob/main/docs/usage.md, as of 2026-09-16). tfsec's README says "our engineering attention will be directed at Trivy going forward" and points to a migration guide (https://github.com/aquasecurity/tfsec/blob/master/README.md, as of 2026-09-16); its last release was 2025-05-02, so a Terraform analyzer choice today is trivy or checkov, both Apache-2.0 with SARIF output (https://trivy.dev/latest/docs/configuration/reporting/ and https://github.com/bridgecrewio/checkov/blob/main/docs/2.Basics/CLI%20Command%20Reference.md, as of 2026-09-16). SARIF 2.1.0 is the OASIS standard all of them target (https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html, as of 2026-09-16).

These analyzers produce findings, not model facts; Warrant relates their SARIF results to obligations, records tool version and result digest in the receipt, and treats a missing tool or truncated report as a failure. For parsing the files, tree-sitter grammars cover Dockerfile, YAML, TOML, and HCL (section 8); the Rust `dockerfile-parser` crate last released in 2024 (0.9.0, 2024-10-31, https://crates.io/crates/dockerfile-parser, as of 2026-09-16).

## 7. Cross-language symbol standards

SCIP is "a language-agnostic protocol for indexing source code" defined by a Protobuf schema with Go and Rust bindings and a `scip` CLI (https://github.com/scip-code/scip/blob/main/README.md, as of 2026-09-16). The repository now lives under the `scip-code` organization; the crates.io `scip` crate (0.10.0, 2026-09-03, Apache-2.0) points there (https://crates.io/crates/scip, as of 2026-09-16), and the Go module and GitHub release are also v0.10.0 (https://github.com/scip-code/scip/releases, as of 2026-09-16).

The schema's core is the `Occurrence`: a typed range, a `symbol` string, `symbol_roles` bit flags (definition, reference, and so on), an optional enclosing range, and diagnostics (https://github.com/scip-code/scip/blob/main/scip.proto, as of 2026-09-16). Symbols follow a grammar the schema states as `<symbol> ::= <scheme> ' ' <package> ' ' (<descriptor>)+ | 'local ' <local-id>`, so a symbol string encodes the indexer, the package and version, and the path of descriptors down to the item. The CLI offers `lint`, `print`, `snapshot`, `stats`, and `test` (https://github.com/scip-code/scip/blob/main/docs/CLI.md, as of 2026-09-16), and the `snapshot` and `test` commands are a ready-made conformance harness for any index Warrant consumes.

| Indexer | Version (as of 2026-09-16) | License | Source |
| --- | --- | --- | --- |
| scip-typescript | 0.4.0, 2025-10-02, depends on `typescript ^5.6.2` | Apache-2.0 | https://www.npmjs.com/package/@sourcegraph/scip-typescript |
| scip-python | 0.6.6, 2025-09-05 | MIT (npm manifest) | https://www.npmjs.com/package/@sourcegraph/scip-python |
| scip-go | v0.2.7, 2026-05-25 | Apache-2.0 | https://github.com/sourcegraph/scip-go |
| rust-analyzer `scip` subcommand | release 2026-09-14 | Apache-2.0 (repository) | https://github.com/rust-lang/rust-analyzer |

The scip-typescript dependency pin is the important row. Its package manifest depends on `typescript ^5.6.2` (https://registry.npmjs.org/@sourcegraph/scip-typescript/0.4.0, as of 2026-09-16), while the `typescript` package's latest release is 7.0.2 (2026-07-08, https://www.npmjs.com/package/typescript, as of 2026-09-16), the line W02 calls native TypeScript 7. scip-typescript therefore indexes with the TypeScript 5 JavaScript compiler, which is not the compiler a TypeScript 7 project builds with. That is a direct conflict with W02's "Resolve the same program the build uses," and it is why Warrant's TypeScript integration reads the compiler itself rather than consuming scip-typescript.

LSIF is SCIP's predecessor. Its 0.6.0 specification is still published (https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/, as of 2026-09-16), but Sourcegraph documents migrating LSIF uploads to SCIP (https://sourcegraph.com/docs/admin/how-to/lsif-scip-migration, as of 2026-09-16) and its indexers emit SCIP. No new integration should target LSIF.

### Should Warrant standardize its symbol layer on SCIP occurrences?

Adopt SCIP as an evidence interchange format, not as Warrant's internal symbol model. For consuming SCIP: four target languages have an indexer that writes it, the Rust bindings are official, the symbol grammar encodes package and version, and the CLI's snapshot tests give conformance for free. Against making it the model: SCIP has no notion of an import edge, a registration, an entrypoint, an effect, or a classification, only occurrences and symbol relationships; it does not distinguish a runtime import from a type-only import, which W02 requires; and its indexers lag their compilers, so an integration that depends only on SCIP inherits that lag.

The practical design: Warrant keeps its own symbol table with a stable identity, populated from a SCIP index (recording the indexer and version as authority) or from a compiler directly. The identity should convert to a SCIP symbol string so indexes and the model can be cross-checked and a future export is cheap. Standardizing the internal representation on SCIP occurrences would be premature.

## 8. tree-sitter as the lowest common denominator

tree-sitter (0.27.0, 2026-08-30, MIT, https://github.com/tree-sitter/tree-sitter, as of 2026-09-16) gives an incremental, error-tolerant concrete syntax tree for any language with a grammar, and the Rust bindings are first-party. Its node API exposes `is_error`, `is_missing`, and `has_error` (https://docs.rs/tree-sitter/latest/tree_sitter/struct.Node.html, as of 2026-09-16), which is what lets a syntax-only pass report a partially parsed file as partial rather than silently dropping it. ast-grep (0.45.3, 2026-08-31, MIT) builds on tree-sitter and supports Bash, Go, HCL, Python, Rust, and YAML among its languages (https://github.com/ast-grep/ast-grep.github.io/blob/main/website/reference/languages.md, as of 2026-09-16), so the vision's pattern-enforced contracts extend to every language listed here without a deeper integration.

| Grammar | crates.io package and version (as of 2026-09-16) | License | Notes |
| --- | --- | --- | --- |
| Rust | tree-sitter-rust 0.24.2, 2026-03-27 | MIT | official, active (https://github.com/tree-sitter/tree-sitter-rust) |
| Python | tree-sitter-python 0.25.0, 2025-09-11 | MIT | official (https://github.com/tree-sitter/tree-sitter-python) |
| Go | tree-sitter-go 0.25.0, 2025-08-29 | MIT | official (https://github.com/tree-sitter/tree-sitter-go) |
| SQL | tree-sitter-sequel 0.3.11, 2025-10-01 | MIT | DerekStride grammar; the crate named `tree-sitter-sql` is a different, unmaintained package (0.0.2, 2021) (https://github.com/DerekStride/tree-sitter-sql) |
| YAML | tree-sitter-yaml 0.7.2, 2025-10-07 | MIT | tree-sitter-grammars org (https://github.com/tree-sitter-grammars/tree-sitter-yaml) |
| TOML | tree-sitter-toml-ng 0.7.0, 2024-12-03 | MIT | the crate named `tree-sitter-toml` (0.20.0, 2022) is a different, stale package (https://github.com/tree-sitter-grammars/tree-sitter-toml) |
| Dockerfile | tree-sitter-dockerfile 0.2.0, 2024-05-09 | MIT | single maintainer (https://github.com/camdencheek/tree-sitter-dockerfile) |
| Bash | tree-sitter-bash 0.25.1, 2025-12-02 | MIT | official (https://github.com/tree-sitter/tree-sitter-bash) |
| HCL | tree-sitter-hcl 1.1.0, 2025-05-16 | Apache-2.0 | tree-sitter-grammars org (https://github.com/tree-sitter-grammars/tree-sitter-hcl) |

Two crate-name traps are worth writing into the spec: for SQL and TOML the obvious crates.io name is the abandoned grammar, and the maintained one has a different name. Pinning by repository URL and digest, not by crate name, avoids both.

### What a syntax-only capability report must admit

A tree-sitter pass sees tokens and structure. It does not see name resolution (which `foo` this is), types, macro or decorator expansion, `cfg` or build-tag evaluation, re-exports, dynamic imports, string-built identifiers, or anything across a file boundary. For SQL it does not see dialect semantics or the schema that results from applying migrations. For YAML and TOML it sees keys and values but not the schema that gives them meaning, so "this is a GitHub Actions workflow" is a classification the inventory declares, not something the parser knows. A syntax-only integration can supply classification hints, declared visibility, unresolved import edges, entrypoint candidates, and pattern-check evidence, and it must report every edge as syntactic, every entrypoint as a candidate, and every file with `has_error` as partially analyzed.

## Recommendations for the spec

### Proposed integration contract

Inputs. A snapshot (tree hash plus the inventory's classification of each file), the subset of files the integration claims, the integration's pinned tool versions, and a declared configuration (feature set, build tags, tsconfig, pyproject, the interface documents to link). An integration never reads outside the snapshot.

Outputs. Model rows in the shared SQLite schema: symbols (with a Warrant identity convertible to a SCIP symbol string), edges, entrypoints, effects, interface symbols, provenance rows for generated files, and a capability report. Every row carries the evidence kind and resolution authority that produced it.

Evidence kinds. `declaration` (from a config, manifest, or interface document), `syntactic` (parsed, unresolved), `resolved` (a compiler or indexer resolved it, with the authority named), `pattern` (matched a declared shape, such as a decorator or a registration call), `provenance` (generated file with producer, version, source digest, and reproducibility result), and `observation` (a live database catalog, a runtime trace, a test receipt, with the observed target recorded). The vision's rule applies to all six: a fact is labeled by the weakest link that produced it.

Resolution authority. Each edge names one of: the language's build tool (`cargo metadata`, `go list`, the TypeScript compiler), an indexer (rust-analyzer scip, scip-go, scip-python, with version), a layout resolver (grimp, ruff analyze graph), or `none` for syntactic edges. Contracts can require a minimum authority; a dependency-direction contract accepts syntactic, a dead-code claim requires resolved.

Capability report fields. Integration name and version; every external tool with version string and digest; the configuration in effect; counts of files claimed, parsed, partially parsed, and unreadable; the list of construct kinds the integration resolves and the list it reports as unresolved (glob imports, macro-generated items, dynamic registration, string-built queries, and so on); the evidence kinds it can emit; and the authority it emitted for each edge kind. A missing tool appears here as a failure, matching W17.

### Build order and why

1. TypeScript/JavaScript first, as planned: it proves the contract against the hardest resolution problem and a compiler Warrant reads directly.
2. Rust second, as the syntactic v1 in section 1: Warrant must check itself, and Rust exercises the full authority ladder (`declaration` from cargo metadata, `syntactic` from syn, optional `resolved` from a SCIP index) in one language.
3. Interface documents and deployment configuration third, together: they turn a single-language graph into an architecture and need only tree-sitter and format parsers, not another compiler.
4. Go fourth: `go list -json` and scip-go make it nearly free once the subprocess-and-JSON pattern exists.
5. Python fifth: it has no compiler authority, and its value is highest where grimp and ruff already run.
6. SQL as a first-class language last, after the effect edge kind exists, because a state-owner contract without write-effect edges is only a declared link.

### Smallest Rust integration for self-hosting

The four-step design in section 1: cargo metadata for the inventory, a syn walker for the module tree and `use` edges with declared visibility, edge qualifiers for `cfg`, and an optional pinned `rust-analyzer scip` pass. That is enough for module ownership, dependency direction, and "no crate outside `warrant-core` imports SQLite directly" contracts on Warrant's own repository, with an honest report that macro-generated items and glob imports are unresolved until the SCIP pass runs.

### Risks

Tool churn. `ra_ap_*` crates are explicitly unstable and republished from every release; ruff's parser crates are versioned 0.0.x with no stability guarantee; rustdoc JSON is nightly-only with an open tracking issue; `ruff analyze graph` is preview-gated; gopls's CLI is documented as experimental. Every one of these should enter Warrant behind a pinned version with the instrument-change discipline the vision already requires.

Indexer lag. scip-typescript targets TypeScript 5 while TypeScript 7 is current; scip-python is a year behind pyright. A SCIP index is evidence with a named, possibly stale, authority, never the same program the build uses.

LSP batch performance. Both rust-analyzer and gopls expose references through a long-lived server designed for interactive use; a batch pass is one request per symbol after cache priming. Prefer the file-producing paths (`rust-analyzer scip`, scip-go, scip-python) and treat LSP as a fallback for single-symbol questions.

License constraints. hadolint is GPL-3.0 and cargo-modules is MPL-2.0; both are usable as separate processes and neither can be vendored into an MIT core. Terraform is BUSL-1.1, which Warrant avoids by parsing HCL rather than linking Terraform. Everything else surveyed is MIT, BSD, or Apache-2.0.

Name traps. The crates.io packages `tree-sitter-sql` and `tree-sitter-toml` are abandoned grammars with the obvious names; pin grammars by repository and digest.
