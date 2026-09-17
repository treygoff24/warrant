# The TypeScript analysis stack for Warrant

Verification date: 2026-09-17. The requested file name and the source spec are dated 2026-09-16, but registry state changed after that date. In particular, Fallow is now 3.27.0 rather than the 3.26.0 recorded by the earlier research.

## Findings at a glance

Warrant should keep two TypeScript lanes with different authority. The fast lane should remain Rust-native: Oxc parses each file, `oxc_semantic` builds its binding graph, and `oxc_resolver` resolves module edges only after a feature-specific parity qualification against the pinned TypeScript compiler. The compiler-authority lane should be a small Node sidecar over the exact `typescript@7.0.2` package's `typescript/unstable/async` client, emitting a versioned, canonical index for Rust to ingest. Spike S1 must approve that choice only after measuring coverage and batch cost. LSP is the fallback protocol, not the preferred batch-index protocol.

This conclusion separates three levels of support:

- Verified means a current registry entry, source file, schema, or first-party release page directly establishes the claim.
- Inference means the sources establish the mechanics, while the consequence for Warrant is an engineering judgment that still needs a spike.
- Could not verify means no current primary source read for this report established the claim.

The most important corrections to the current spec are these:

1. TypeScript 7.0.2 is the stable native compiler and installs `tsc`. `@typescript/native-preview` still exists, but its `latest` tag is a development build and installs `tsgo`. It is not the stable package. The TypeScript 7 package contains `unstable/sync` and `unstable/async` API exports, but Microsoft still describes 7.0 as having no supported programmatic API. [`typescript@7.0.2` registry record](https://registry.npmjs.org/typescript/7.0.2), [`@typescript/native-preview` registry record](https://registry.npmjs.org/@typescript%2fnative-preview), [TypeScript 7 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
2. The published `@sourcegraph/scip-typescript@0.4.0` package depends on `typescript@^5.6.2`, while the default branch's `package.json`, still labeled 0.4.0, now depends on `typescript@^6.0.3`. The published artifact is therefore not a TypeScript 6 or 7 authority unless Warrant deliberately overrides and qualifies its dependency graph. [`0.4.0` registry record](https://registry.npmjs.org/%40sourcegraph%2fscip-typescript/0.4.0), [default-branch `package.json`](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/package.json)
3. `oxc_resolver` documents tsconfig paths, `baseUrl`, project references, and package `exports` and `imports`, but it does not expose named `node16` or `nodenext` modes and does not claim blanket parity with TypeScript. Its declaration resolver explicitly claims the `bundler` algorithm. Warrant must qualify actual edges rather than convert that feature list into an authority claim. [`oxc_resolver` README](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/README.md), [`oxc_resolver` TypeScript declarations](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/napi/index.d.ts)

## TypeScript 7 packages and access surfaces

### Published packages and version lines

The npm registry contained these versions on 2026-09-17. The version-line rows use the highest stable patch visible in the registry, not a dist-tag inference. [`typescript` registry](https://registry.npmjs.org/typescript), [`@typescript/native-preview` registry](https://registry.npmjs.org/@typescript%2fnative-preview), [`@typescript/typescript6` registry](https://registry.npmjs.org/@typescript%2ftypescript6)

| Purpose | Package and exact version | Dist-tags and executable | Status for Warrant |
| --- | --- | --- | --- |
| Current stable compiler | `typescript@7.0.2` | `latest=7.0.2`, `rc=7.0.1-rc`, `next=7.1.0-dev.20260917.1`; binary `tsc` | Stable compiler and parity oracle |
| Legacy native preview | `@typescript/native-preview@7.0.0-dev.20260707.2` | `latest=7.0.0-dev.20260707.2`, `beta=7.0.0-dev.20260421.2`; binary `tsgo` | Historical preview, do not pin for a new integration |
| JavaScript API compatibility wrapper | `@typescript/typescript6@6.0.2` | `latest=6.0.2`; binary `tsc6` | Wrapper for tools that still need the JavaScript compiler API |
| Last JavaScript compiler release | `typescript@6.0.3` | highest stable 6.x package | Actual compiler selected by the wrapper's `@typescript/old: npm:typescript@^6` dependency when freshly resolved |
| Last 5.x compiler release | `typescript@5.9.3` | highest stable 5.x package | Compatibility baseline only |
| Separate API package | `@typescript/api` | no package published at the registry URL | Do not put this name in the lock |

The stable TypeScript package uses optional platform packages for its native executable and exports only a version object at the package root. Its programmatic surfaces are explicitly named `typescript/unstable/sync`, `typescript/unstable/async`, `typescript/unstable/fs`, `typescript/unstable/proto`, and several `unstable/ast` paths. That export map is verified in the published 7.0.2 package. [`typescript@7.0.2` registry record](https://registry.npmjs.org/typescript/7.0.2)

The compatibility package is a thin wrapper. Its `lib/typescript.js` requires `@typescript/old`, its declaration file re-exports that package, and `tsc6` loads the old compiler's CLI. The wrapper itself is 6.0.2, while a fresh resolution of `^6` can install 6.0.3. Warrant must record both identities from the resolved lockfile rather than treating the wrapper version as the compiler version. [`@typescript/typescript6@6.0.2` registry record](https://registry.npmjs.org/%40typescript%2ftypescript6/6.0.2)

### What is stable

The compiler, builder, watch mode, and LSP server are supported TypeScript 7 surfaces. Microsoft's release announcement reports typical full-build improvements of 8 to 12 times on its measured projects and publishes results for large repositories, but those numbers are upstream benchmarks rather than a Warrant budget. The same announcement says TypeScript 7.0 does not ship with a supported API and expects the new API in 7.1. [TypeScript 7 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)

The apparent contradiction between "no API" and the package's `unstable` exports is only a support distinction. The code is shipped and callable, but its name, source comments, and TypeScript's release statement do not promise a stable contract. Warrant can isolate and pin it for a spike; it must not describe it as stable.

### LSP from a Rust process

The native executable starts its language server with `tsc --lsp --stdio`. The server currently rejects non-stdio LSP startup. It registers standard document diagnostics, definition, type definition, implementation, references, document symbols, workspace symbols, call hierarchy, and semantic-token handlers. [`cmd/tsgo/lsp.go`](https://raw.githubusercontent.com/microsoft/typescript-go/main/cmd/tsgo/lsp.go), [`internal/lsp/server.go`](https://raw.githubusercontent.com/microsoft/typescript-go/main/internal/lsp/server.go)

A Rust LSP client can therefore obtain:

- type-aware diagnostics for opened or requested documents;
- definition and type-definition locations for a position;
- references and implementations for a position;
- a document-symbol outline and workspace-symbol search results;
- semantic tokens and call-hierarchy edges.

It cannot obtain a standard whole-program AST, complete type graph, or a bulk map of every import's compiler resolution. The LSP standard is request-oriented. A Warrant indexer would need to discover files, identify positions worth querying, issue many requests, and canonicalize duplicate and unordered results. It can ask for the definition of an import token, but that is not equivalent to `--traceResolution`: it omits failed candidates, conditions, package identifiers, and the compiler's reason for an unresolved edge. This is an inference from the registered handlers and the standard response shapes, not a benchmark result.

Failure is explicit at the transport boundary: startup or JSON-RPC failure, a server error response, a missing document result, or an incomplete traversal. Warrant must count any missing response in its completeness facet. It must also initialize one workspace with fixed roots and settings and prohibit automatic package installation, because the server source includes an npm-install callback for typings. [`cmd/tsgo/lsp.go`](https://raw.githubusercontent.com/microsoft/typescript-go/main/cmd/tsgo/lsp.go)

### The shipped unstable API and RPC transport

The current TypeScript source contains both async and sync clients. The async client spawns `tsc --api --async` and speaks JSON-RPC over stdio, or connects to a Unix-domain socket. The Go server also supports a named pipe or Unix-domain socket. The sync client uses a MessagePack tuple protocol over blocking pipes. [`async/client.ts`](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/async/client.ts), [`cmd/tsgo/api.go`](https://raw.githubusercontent.com/microsoft/typescript-go/main/cmd/tsgo/api.go)

`libsyncrpc` is historical, not the current runtime dependency. Microsoft's 2025 preview post described a Rust native module named `libsyncrpc`. The published 7.0.2 package instead ships a pure-JavaScript `SyncRpcChannel` replacement and warns in source that the sync protocol is unversioned and requires client and server from the same tree. [native preview announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-native-previews/), [`syncChannel.ts`](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/syncChannel.ts)

The separate proposed packages `@typescript/api` and `@typescript/ast` were consolidated into the compiler package to prevent client/server version skew. No `@typescript/api` package was published. The public import paths are now the `unstable` subpaths of the exact compiler package. [consolidation pull request](https://github.com/microsoft/typescript-go/pull/3430), [`@typescript/api` registry endpoint](https://registry.npmjs.org/@typescript%2fapi)

The async API is materially better suited to batch indexing than LSP. Its current source exposes snapshots and projects; source-file names and binary ASTs; syntactic, bind, semantic, suggestion, declaration, program, global, and config diagnostics; symbol lookup by position or AST node; declared and inferred types; symbol declarations and value declarations; aliased symbols and module exports; and references to a symbol within a file. [`async/api.ts`](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/async/api.ts), [`proto.generated.ts`](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/proto.generated.ts)

What it does not currently expose is just as important. The generated method list has no `resolveModuleName` or per-import resolved-module result. `SourceFileMetadata` contains library and module-format metadata, not resolved imports. A sidecar can infer many binding targets by resolving imported aliases to their declarations, but Warrant should still use `tsc --traceResolution` as the resolution oracle. [`proto.generated.ts`](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/proto.generated.ts)

A Rust process could implement the async JSON-RPC wire protocol directly. That is technically possible because the backend is out of process and the wire is JSON-RPC. It is not recommended for v1: the generated protocol is unstable, the object-handle lifecycle is nontrivial, ASTs use a separate binary encoding, and the official JavaScript client is released in the same package as the server. A small sidecar converts that unstable, version-coupled surface into Warrant's own stable JSON or protobuf index.

### TypeScript 6 in the transition

TypeScript 6 is the final JavaScript-based major and the bridge between 5.9 and the Go implementation. Microsoft says it remains API-compatible with 5.9 while aligning compiler behavior with 7.0. [TypeScript 6 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-6-0/)

The migration deprecations include `target: es5`, `downlevelIteration`, `moduleResolution: node` or `node10`, AMD/UMD/SystemJS module output, `baseUrl` as a lookup root, false values for `esModuleInterop` and `allowSyntheticDefaultImports`, `alwaysStrict: false`, legacy `module` namespace syntax, import assertions, and `no-default-lib` directives. TypeScript 6 removes `moduleResolution: classic` and `outFile`. It adds `stableTypeOrdering` to expose TypeScript 7's deterministic type and symbol ordering while projects still run the JavaScript compiler. [TypeScript 6 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-6-0/)

For Warrant, TypeScript 6 is an API compatibility tool, not the resolution authority for a TypeScript 7 project. A report generated by 6.0.3 must be labeled with that compiler identity and may corroborate, but cannot silently satisfy an obligation requiring TypeScript 7 behavior.

## Four ways to build the TypeScript program model

### Capability comparison

| Route | Type-level facts | Binding and syntax facts | Resolution | Large-monorepo behavior | User install burden | Failure mode |
| --- | --- | --- | --- | --- | --- | --- |
| Drive `tsc --lsp --stdio` from Rust | Type-aware definitions, references, implementations, diagnostics, hover-adjacent data | Document outlines and semantic tokens, not a full AST or bulk symbol table | Only indirectly through navigation; no trace or bulk resolved-edge API | Native server is fast and multithreaded, but a complete index requires many position requests. The extraction cost is unresolved and must be measured | Exact TypeScript platform package; no Node sidecar code | JSON-RPC/startup error, missing response, unopened project, or incomplete enumeration |
| Node sidecar over `typescript/unstable/async` | Program/checker types, diagnostics, declarations, aliases, exports, per-file references | Full source file list and binary AST access | Alias declarations can identify many targets; use `--traceResolution` for authoritative edges | One snapshot and batch methods avoid much LSP request overhead. Whole-repository cost is unresolved and belongs in S1 | Exact `typescript` package and Node, both already needed for parity and npm instruments | Sidecar nonzero exit, API method error, version mismatch, malformed index, or incomplete capability report |
| Consume `scip-typescript` output | Compiler-backed occurrences and symbol relationships, but not the compiler's general type graph | Definitions and references, no full syntax tree | Symbol identities imply many resolved relationships, but SCIP does not record TypeScript's full resolution trace or failed candidates | Project recursion and Yarn/pnpm workspaces are supported; upstream documents possible Node heap exhaustion on large codebases | Node, `@sourcegraph/scip-typescript`, project dependencies, and a protobuf reader | Missing project, config diagnostics, OOM, zero indexed files, malformed/truncated protobuf |
| Build with Oxc plus resolver parity | No inferred TypeScript types | Full syntax and per-file scopes, symbols, and references | `oxc_resolver`, authoritative only for qualified feature sets | Rust-native, parallelizable, no sidecar on the warm fast path | Warrant binary only for the fast path; TypeScript is still needed when qualifying | Parse errors, unresolved edges, unsupported constructs, or project marked unqualified after a parity disagreement |

The performance cells distinguish verified upstream mechanics from Warrant inference. TypeScript publishes native compiler benchmarks, and scip-typescript documents its OOM controls, but no Warrant-shaped benchmark compared these four complete indexing strategies. [TypeScript 7 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/), [`scip-typescript` README](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/README.md)

### Route A: LSP

LSP is the strongest language-neutral fallback because a Rust client can use a standard protocol and compiler-authority navigation. It is the weakest batch contract. `documentSymbol` is an outline, `references` and `definition` start from positions, and diagnostics arrive as document reports or pushed notifications. None yields a complete compiler `Program` in one bounded operation. The S1 corpus must count requests, bytes, missing results, and duplicate locations, not just wall time.

Determinism is achievable only after normalization. Warrant should sort locations by normalized path and byte range, identify the exact workspace roots and compiler package, and discard presentation-only ordering. A timeout or a file that never reaches a diagnostic/reference result is incompleteness, never an empty answer.

### Route B: a Node sidecar over the compiler API

This is the recommendation for compiler-authority augmentation. The sidecar should import `API` from `typescript/unstable/async`, open named tsconfig projects in one snapshot, enumerate source files, and emit only Warrant's needed facts: project identity, diagnostics, exported symbols, declaration locations, aliases, module exports, and sampled or complete reference locations. It should not expose TypeScript's remote object handles to Rust. The emitted index needs its own schema version and must be sorted before hashing.

This design contains the unstable API behind one replaceable process. Exact pinning makes source and server match; a Warrant adapter update is then an explicit instrument upgrade. It also allows a later direct Rust JSON-RPC client without changing the model schema.

The limitation is resolution. The sidecar should run or accompany the same exact compiler's `--traceResolution` output rather than claim that declaration lookup is a complete resolution record. If the sidecar's compiler-level reference extraction misses S1's hand-labeled symbols, Warrant keeps binding-level references and reports the compiler capability as unavailable.

### Route C: SCIP from `scip-typescript`

`scip-typescript` writes protobuf SCIP. Its command defaults to `index.scip`, recursively indexes `projectReferences`, and has explicit Yarn and pnpm workspace discovery. It serializes a metadata record followed by document records. [`src/main.ts`](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/src/main.ts), [`CommandLineOptions.ts`](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/src/CommandLineOptions.ts)

SCIP records documents, occurrences, symbol information, relationships, diagnostics-independent metadata, and optional source text. It is designed for go-to-definition, references, and implementations, not as a full TypeScript AST or type-checker serialization. [SCIP protobuf schema](https://raw.githubusercontent.com/sourcegraph/scip/main/scip.proto)

Rust can read the format with `scip@0.10.0`, which provides generated protobuf types and uses `protobuf@3.7.2`; `protobuf::Message::parse_from_bytes` handles decoding. The crate's MSRV is 1.81.0. [`scip` registry record](https://crates.io/api/v1/crates/scip), [`scip@0.10.0` crate archive](https://static.crates.io/crates/scip/scip-0.10.0.crate)

The current publishing mismatch prevents recommending SCIP as the default authority. The npm artifact is `@sourcegraph/scip-typescript@0.4.0` with `typescript@^5.6.2`. The default branch is also labeled 0.4.0 but pins `typescript@^6.0.3`, requires newer Node versions, and has not appeared as a distinct npm release. Warrant can either pin the published tarball and accept 5.x semantics, or build a commit as a custom instrument and record its commit plus dependency lock. It must not record only `0.4.0` and assume those two artifacts are equivalent. [`0.4.0` registry record](https://registry.npmjs.org/%40sourcegraph%2fscip-typescript/0.4.0), [default-branch `package.json`](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/package.json)

### Route D: Oxc with parity qualification

This is the recommended fast baseline. It supplies the syntax and binding graph Warrant needs for most architecture contracts without a resident Node process. It cannot answer type-derived reference questions, and the resolver is not authoritative until qualification. Those limits belong in the capability report rather than in prose only.

### Recommendation and deciding spike

S1 decides the compiler-authority route. Its preferred candidate should now be the exact TypeScript 7.0.2 async API through a small Node indexer, not a hypothetical `@typescript/api` package. Compare that sidecar against LSP and the pinned SCIP artifact on the frozen corpus. Measure symbol precision and recall, diagnostics coverage, declaration and reference completeness, request count, bytes, peak memory, and canonical-index stability over repeated runs. Do not let TypeScript 6 or SCIP results qualify as TypeScript 7 compiler authority.

S6 separately decides whether `oxc_resolver` may carry `parity-qualified` authority for each project's actual resolution feature set.

## The Oxc crates and resolution boundary

### Version set, MSRV, and pinning

The five compiler-tooling crates are released as one workspace version. The workspace manifest assigns 0.150.0 to the parser, AST, semantic, span, allocator, and most sibling crates, with Rust 1.96.0 as the workspace MSRV. `oxc_resolver` is a separate repository and release line at 11.24.3 with Rust 1.95.0. [Oxc workspace manifest](https://raw.githubusercontent.com/oxc-project/oxc/main/Cargo.toml), [`oxc_resolver` manifest](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/Cargo.toml)

| Crate | Exact version | MSRV | Release source |
| --- | --- | --- | --- |
| `oxc_parser` | `0.150.0` | `1.96.0` | [crates.io](https://crates.io/api/v1/crates/oxc_parser) |
| `oxc_ast` | `0.150.0` | `1.96.0` | [crates.io](https://crates.io/api/v1/crates/oxc_ast) |
| `oxc_semantic` | `0.150.0` | `1.96.0` | [crates.io](https://crates.io/api/v1/crates/oxc_semantic) |
| `oxc_span` | `0.150.0` | `1.96.0` | [crates.io](https://crates.io/api/v1/crates/oxc_span) |
| `oxc_allocator` | `0.150.0` | `1.96.0` | [crates.io](https://crates.io/api/v1/crates/oxc_allocator) |
| `oxc_resolver` | `11.24.3` | `1.95.0` | [crates.io](https://crates.io/api/v1/crates/oxc_resolver) |

Oxc's core crates have been releasing roughly weekly. The parser changelog shows 0.141.0 on 2026-07-20, 0.142.0 on 2026-07-27, 0.143.0 on 2026-08-03, 0.144.0 on 2026-08-10, 0.149.0 on 2026-09-07, and 0.150.0 on 2026-09-14. Several of those minor releases contain explicitly labeled breaking AST or parser API changes. [`oxc_parser` changelog](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_parser/CHANGELOG.md), [`oxc_ast` changelog](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_ast/CHANGELOG.md)

A consumer should use exact requirements for the whole Oxc set, for example `oxc_parser = "=0.150.0"`, and commit one Cargo lock resolution. It should upgrade all Oxc workspace crates together after corpus qualification. A caret requirement on a pre-1.0 crate would permit a newer patch inside 0.150, but the more important protection is preventing a piecemeal workspace mix and making every instrument upgrade deliberate. `oxc_resolver` should be pinned independently as `=11.24.3`.

### What `oxc_semantic` establishes

`oxc_semantic` builds an AST-node store, a scope tree, symbol tables, identifier references, class metadata, and optionally a control-flow graph and JSDoc data. Its `Semantic` API exposes scoping, symbol references, symbol scopes, unresolved references, and counts of nodes, scopes, symbols, and references. [`oxc_semantic` library source](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/src/lib.rs), [`oxc_semantic` README](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/README.md)

The boundary is one parsed `Program`. `SemanticBuilder::build` takes a single `oxc_ast::Program` and has no project graph, filesystem, package tree, or tsconfig input. It can identify import and export syntax and bind references within that file, but it cannot decide which other file an import specifier names or perform TypeScript type inference across files. That negative conclusion is an inference directly from the builder API, and it matches the need for a separate resolver. [`SemanticBuilder::build`](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/src/builder.rs)

For Warrant, `symbol_level: binding` is accurate only after `lang-ts` joins the per-file semantic result to a resolved module edge and follows export and re-export chains. `oxc_semantic` alone does not produce that repository-level graph.

### What `oxc_resolver` supports

The resolver is a Rust port of webpack's `enhanced-resolve`, `tsconfig-paths-webpack-plugin`, and `tsconfck`. Its first-party documentation establishes the following support. [`oxc_resolver` README](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/README.md), [`TsConfig` source](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/src/tsconfig.rs)

| Feature | Verified behavior | Qualification consequence |
| --- | --- | --- |
| `paths` | Parses and compiles `compilerOptions.paths` mappings | Include exact, wildcard, fallback-array, extends, and missing-target cases |
| `baseUrl` | Resolves it and paths relative to the canonical config; can use it as a fallback root | Test TypeScript 6 and 7's changed `baseUrl` semantics explicitly |
| Project references | Loads referenced configs, resolves referenced ownership, and can auto-discover the config that owns a file | Include solution-style roots, nested references, conflicting includes, and per-reference options |
| Package `exports` | Uses configurable `exportsFields` and ordered `conditionNames` | Test import versus require conditions, types conditions, arrays, null targets, patterns, and self-references |
| Package `imports` | Uses configurable `importsFields` | Test package-local `#` mappings and conditions |
| `bundler` | `resolveDtsSync` and `resolveDtsAsync` explicitly say they use TypeScript's bundler algorithm | This claim applies to the declaration resolver, not every generic resolver call |
| `node16` | No named mode is exposed in the options or tsconfig model | Unverified until S6 configures conditions and passes the corpus |
| `nodenext` | No named mode is exposed in the options or tsconfig model | Unverified until S6, especially because `nodenext` changes with Node behavior |

The generic resolver requires the caller to choose ESM or CommonJS condition names, such as `node,import` or `node,require`. It also has defaults derived from enhanced-resolve rather than from the active TypeScript `moduleResolution` mode. The TypeScript handbook documents material differences among `node16`, `node18`, `nodenext`, and `bundler`, including file-format detection and extension requirements. [`ResolveOptions` source](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/src/options.rs), [TypeScript modules reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html)

One current divergence is directly visible in source. `oxc_resolver` still tries `baseUrl/specifier` when no `paths` mapping matches, while TypeScript 6 deprecated `baseUrl` and stopped treating it as a module-resolution lookup root in preparation for TypeScript 7. `lang-ts` must disable or compensate for that fallback before parity is possible on such a config. [`oxc_resolver` tsconfig resolution](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/src/tsconfig.rs), [TypeScript 6 `baseUrl` change](https://devblogs.microsoft.com/typescript/announcing-typescript-6-0/)

The current primary sources do not publish a list of remaining `tsc` divergences or claim zero disagreement. The resolver's changelog shows recent fixes for tsconfig ownership, project-reference priority, `baseUrl` and `paths` anchoring, referenced-project `allowJs`, and package export behavior. That history is evidence of active convergence, not present parity. [`oxc_resolver` changelog](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/CHANGELOG.md)

### Resolution parity qualification

S6 should produce an edge-level comparison, not an aggregate percentage alone.

1. Freeze the corpus, dependency trees, exact TypeScript package, exact `oxc_resolver`, operating-system path semantics, symlink policy, and every tsconfig and package manifest digest.
2. Enumerate every literal module-bearing construct with Oxc: ESM imports and re-exports, `import type`, `import = require`, literal `require`, literal dynamic `import`, triple-slash type references, and config project references. Record importer, specifier, syntax kind, type-only flag, and byte span.
3. Run `typescript@7.0.2` with `--noEmit --pretty false --traceResolution` for every owning tsconfig. For solution-style builds, run each referenced project under its own config so the comparison does not guess which trace owns an importer. TypeScript documents `traceResolution` as the switch that prints how each processed module was resolved. [TypeScript `traceResolution` reference](https://www.typescriptlang.org/tsconfig/traceResolution.html)
4. Parse the trace with a versioned adapter. Normalize paths according to the project's `preserveSymlinks` setting, but retain both the reported and canonical path for diagnosis. Reject any trace block the adapter cannot bind to exactly one enumerated edge.
5. Resolve the same edge with `oxc_resolver` using the owning config, importer format, import or require conditions, extension set, package fields, and project references derived by `lang-ts`. Record target, external-package identity, or unresolved reason.
6. Compare every in-scope edge by `(project, importer, span, specifier, resolution mode)`. A pass requires zero differences in resolved versus unresolved outcome and zero differences in the final target after the defined path normalization. Extra, missing, ambiguous, or unparsed edges are failures. Package identity and chosen export condition should be compared when both instruments expose them; absence is recorded as an observability gap.
7. Run the qualification twice from clean Warrant caches. Canonical edge sets must match between runs. Timings are observations and are not part of the model digest.
8. Emit a feature-coverage matrix. A repository receives `parity-qualified` only when every resolution feature it uses appears in a zero-disagreement corpus row for the same platform and option family.

If any project disagrees, that project is `unqualified`. Its analysis facet is incomplete with `resolution-unqualified`; Warrant should either route that project through compiler-produced resolution or stop the gate. A signed ruling may accept one named configuration feature as the spec allows, but a project-level disagreement must never be hidden inside a repository-wide agreement percentage.

## Structural pattern matching

### Versions and library boundary

| Component | Exact version | MSRV or grammar ABI | Role |
| --- | --- | --- | --- |
| `ast-grep-core` | `0.45.3` | Rust `1.88.0` | Nodes, matchers, patterns, edits |
| `ast-grep-config` | `0.45.3` | Rust `1.88.0` | YAML rule parsing, constraints, transforms, metadata |
| `ast-grep-language` | `0.45.3` | Rust `1.88.0` | Built-in language enum and grammar bindings |
| `tree-sitter` | `0.27.0` | Rust `1.90`, parser ABI 13 through 15 | Parser runtime |
| `tree-sitter-typescript` | `0.23.2` | generated parser ABI 14 | TypeScript and TSX grammars |

The crate versions and MSRVs come from crates.io. [`ast-grep-core`](https://crates.io/api/v1/crates/ast-grep-core), [`ast-grep-config`](https://crates.io/api/v1/crates/ast-grep-config), [`ast-grep-language`](https://crates.io/api/v1/crates/ast-grep-language), [`tree-sitter`](https://crates.io/api/v1/crates/tree-sitter), [`tree-sitter-typescript`](https://crates.io/api/v1/crates/tree-sitter-typescript)

Ast-grep's own API reference says its Rust API is not stable. Warrant should therefore exact-pin the three ast-grep crates as one set and qualification-test its rule fixtures on upgrade. [ast-grep API reference](https://ast-grep.github.io/reference/api)

### YAML rule contract

An ast-grep rule document has metadata plus a `rule` object. The atomic fields include `pattern`, `kind`, and `regex`; relational rules include `has`, `inside`, `follows`, and `precedes`; composite rules include `all`, `any`, and `not`. `constraints` is a top-level map from a single metavariable name, without `$`, to another rule object and is applied after the main rule. [rule essentials](https://ast-grep.github.io/guide/rule-config), [rule reference](https://ast-grep.github.io/reference/rule), [YAML reference](https://ast-grep.github.io/reference/yaml)

The relevant semantics are:

- `pattern` parses example source with the selected grammar and supports `$NAME`, `$$NAME`, and `$$$NAME` metavariables;
- `kind` matches a named tree-sitter node kind;
- `regex` applies a Rust regular expression to the text of the candidate node and should be paired with a structural restriction;
- `has` requires a matching descendant, with optional `field` and `stopBy` controls;
- `inside` requires a matching ancestor, also with `field` and `stopBy` controls;
- `constraints` filters the node bound to one metavariable after the main pattern has matched.

Ast-grep binds a language by implementing its `Language` trait: node-kind names map to grammar kind IDs, field names map to field IDs, and patterns are parsed through that language's tree-sitter parser. `ast-grep-language` binds `SupportLang::TypeScript` and `SupportLang::Tsx` to separate grammar functions. [`ast-grep-language` source](https://static.crates.io/crates/ast-grep-language/ast-grep-language-0.45.3.crate)

### Tree-sitter 0.27 traps

There are three traps for an embedded Rust consumer.

1. ABI compatibility and crate compatibility are separate. Tree-sitter 0.27 accepts generated parser ABI 13 through 15. `Parser::set_language` rejects a parser outside that range or one marked not parseable. [Tree-sitter ABI table](https://tree-sitter.github.io/tree-sitter/using-parsers/7-abi-versions.html), [`tree-sitter@0.27.0` crate archive](https://static.crates.io/crates/tree-sitter/tree-sitter-0.27.0.crate)
2. `Language::name()` can return `None` for older generated parsers. The TypeScript and TSX parsers in `tree-sitter-typescript@0.23.2` are ABI 14, while their declared grammar names are `typescript` and `tsx`. Warrant must select the grammar explicitly from the file classification; it must not dispatch by `Language::name()` or confuse package aliases such as `ts` with the embedded grammar name. [`tree-sitter` Rust binding](https://static.crates.io/crates/tree-sitter/tree-sitter-0.27.0.crate), [`tree-sitter-typescript` parser source](https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/typescript/src/parser.c), [`tree-sitter-typescript` metadata](https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/tree-sitter.json)
3. Grammar node-kind and field names are part of Warrant's rule behavior even when the runtime ABI still loads. A grammar upgrade can change a YAML `kind` or `field` match without a tree-sitter runtime error. Pin `tree-sitter-typescript@0.23.2`, retain positive and negative fixtures for every shipped rule, and treat a grammar upgrade as an instrument upgrade.

### Direct tree-sitter and Semgrep-style patterns

Direct tree-sitter is the smaller abstraction if Warrant needs only CST parsing and S-expression queries. It provides node kinds, fields, ranges, incremental parsing, and query captures. It does not provide ast-grep's source-like metavariable patterns, YAML relational/composite rules, metavariable constraints, or rewrite machinery. Warrant would have to design and maintain those semantics itself. [Tree-sitter getting started](https://tree-sitter.github.io/tree-sitter/using-parsers/1-getting-started.html), [how ast-grep works](https://ast-grep.github.io/advanced/how-ast-grep-works)

Semgrep-style rules are more familiar to some users and include source patterns, `$X` metavariables, ellipses, `pattern-inside`, negation, metavariable regexes, and selected semantic analyses. The current open-source distribution is a separate CLI installed through Python packaging and is LGPL-2.1, not an embeddable Rust crate with Warrant's dependency profile. [Semgrep pattern syntax](https://docs.semgrep.dev/writing-rules/pattern-syntax), [Semgrep rule syntax](https://docs.semgrep.dev/writing-rules/rule-syntax), [Semgrep repository license](https://raw.githubusercontent.com/semgrep/semgrep/develop/LICENSE)

Ast-grep is the better fit for v1 because its Rust crates embed directly, its YAML already expresses the requested structural relations, and its language layer is explicit. That recommendation does not imply semantic equivalence with Semgrep. A Warrant pattern contract should state that it is a syntax-tree predicate unless a separate compiler fact participates.

## TypeScript ecosystem instruments

### Version and schema pins

Registry state on 2026-09-17 produced the following initial lock candidates. Each is an exact version rather than a range or moving tag.

| Instrument | Exact package version | Machine output contract | Determinism and lock treatment |
| --- | --- | --- | --- |
| TypeScript | `typescript@7.0.2` | compiler diagnostics text, `--traceResolution` text, and unstable API protocol | Pin platform package and compiler package, registry integrity, invocation, config digests, and adapter version |
| Knip | `knip@6.36.0` | JSON reporter TypeScript interfaces in `packages/knip/src/reporters/json.ts`; no formal JSON Schema verified | Static findings should repeat for fixed inputs, but ordering is not promised; pin full npm resolution and canonicalize |
| Fallow | `fallow@3.27.0` | generated Draft 7 `docs/output-schema.json`, typed npm exports, JSON or SARIF | Upstream promises deterministic findings and stable fingerprints; pin npm integrity, platform binary digest, schema digest, args, and config |
| dependency-cruiser | `dependency-cruiser@18.3.1` | Draft 7 configuration and cruise-result schemas | Static graph is reproducible only with fixed package tree, resolver options, platform, and non-dynamic config; canonicalize |
| StrykerJS | `@stryker-mutator/core@10.0.0` | `mutation-testing-report-schema@3.8.4` | Test execution, timeouts, and scheduling are observational; pin runner, plugins, schema, config, and test command |
| Vitest | `vitest@5.0.1` | source-level `JsonTestResults` interface, Jest-like JSON | Outcomes may repeat, timestamps and durations do not; pin runtime, config, environment class, args, and adapter |
| Jest | `jest@30.5.1` | source-level `FormattedTestResults` interface | Same limitation as Vitest; pin project packages, config, runtime, args, and adapter |

Version sources: [`typescript`](https://registry.npmjs.org/typescript/7.0.2), [`knip`](https://registry.npmjs.org/knip), [`fallow`](https://registry.npmjs.org/fallow), [`dependency-cruiser`](https://registry.npmjs.org/dependency-cruiser), [`@stryker-mutator/core`](https://registry.npmjs.org/%40stryker-mutator%2fcore), [`vitest`](https://registry.npmjs.org/vitest), [`jest`](https://registry.npmjs.org/jest).

The instruments lock needs more than a top-level version string. For each npm instrument it should record the exact package name and version, registry integrity, resolved dependency-lock digest, executable or platform-binary digest, Node version range and actual Node version, invocation array, working-directory unit, config digest, output-schema identity and digest, Warrant adapter version, permitted exit codes, and environment-name policy. A range such as `^6.36.0` or a mutable GitHub action tag is not a pin.

The report artifact should retain the raw output digest. The adapter may also emit a canonical finding set with paths normalized and arrays sorted, but the receipt must distinguish raw evidence from normalized evidence. Timing fields belong in the receipt as observations, not in the semantic finding digest.

### Knip

Knip 6.36.0's JSON reporter emits an object with one `issues` array. Each entry has a relative `file` and optional arrays for binaries, catalog findings, cycles, dependencies, development dependencies, duplicates, enum members, exports, files, namespace members, types, unlisted dependencies, and unresolved imports. Items can carry `name`, `namespace`, `kind`, `specifier`, byte position, line, and column; cycles and duplicates are nested arrays. [`knip` package manifest](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/package.json), [JSON reporter source](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/src/reporters/json.ts), [issue types](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/src/types/issues.ts)

No standalone JSON Schema was verified. The source TypeScript interfaces are the output contract for this pin, so Warrant's adapter fixtures should carry representative examples for every issue family it imports. The package currently depends on `oxc-parser@^0.148.0` and exactly `oxc-resolver@11.24.2`; this is separate from Warrant's own Oxc pins and is another reason to record the resolved npm tree, not only `knip@6.36.0`. [`knip` registry](https://registry.npmjs.org/knip)

Knip's analysis is static, so the finding set should be deterministic after freezing source, config, workspace discovery, package tree, Node, and platform path behavior. The source does not promise canonical array order. Warrant should sort by issue kind, normalized path, location, and name before comparing upgrades, while retaining the raw report digest.

### Fallow

Fallow is the `fallow` npm package published from the `fallow-rs/fallow` repository. Version 3.27.0 was published on 2026-09-17 and supersedes the 3.26.0 lead in the earlier research. The Rust workspace uses Oxc parser, AST, semantic, span, syntax, allocator, and resolver crates, so describing it as Oxc-based is verified. [`fallow@3.27.0` registry record](https://registry.npmjs.org/fallow/3.27.0), [Fallow workspace manifest](https://raw.githubusercontent.com/fallow-rs/fallow/main/Cargo.toml)

The package installs `fallow`, `fallow-lsp`, and `fallow-mcp`. Its static analyzer reports dead code, cycles, duplication, complexity and health, architecture boundaries, and related evidence. Its README states that static runs are deterministic and use stable fingerprints. Optional type-aware analysis is a separate checker-backed companion and must not be confused with the default Oxc-only analysis. [Fallow README](https://raw.githubusercontent.com/fallow-rs/fallow/main/README.md)

`--format json --quiet` emits one typed JSON document. Object envelopes use a top-level `kind`; current schema branches include combined, dead-code, dupes, health, audit, trace, security, and other command-specific outputs. A Code Climate report is a bare array, and an error envelope is identified by `error: true`. The generated Draft 7 schema is the authoritative shape. [Fallow output schema](https://raw.githubusercontent.com/fallow-rs/fallow/main/docs/output-schema.json)

Warrant should pin `fallow@3.27.0` plus the actual installed platform binary digest and the schema digest. It should accept exit 0 as clean and exit 1 as a successful run with findings; exit 2 is a tool failure with a JSON error envelope. Other documented exit codes are command-specific and must be allowed only for the pinned invocation. The adapter should import stable finding fingerprints, scope, locations, evidence completeness, and command kind without importing the heuristic health score as a hard architectural fact. [Fallow README](https://raw.githubusercontent.com/fallow-rs/fallow/main/README.md)

### dependency-cruiser

Dependency-cruiser 18.3.1 config accepts JSON or a JavaScript module. Its policy sections are `forbidden`, `allowed`, `required`, `allowedSeverity`, `extends`, and `options`. Individual dependency rules use `from` and `to` conditions, with names and severities on forbidden rules and a `module` selector on required rules. [package manifest](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/package.json), [rules reference](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/doc/rules-reference.md), [configuration schema](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/schema/configuration.schema.json)

Its JSON reporter serializes the cruise result directly. The result schema requires `summary` and `modules`; each module carries a source path, validity, dependencies, and optional resolution, reachability, rule, license, and classification details. [JSON reporter](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/report/json.mjs), [cruise-result schema](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/schema/cruise-result.schema.json)

For census import, Warrant should import observed modules, dependency edges, dependency types, resolution failures, and violated-rule provenance. A JavaScript config can execute code and therefore weakens reproducibility. Prefer a JSON config for Warrant-owned qualification, or record the JavaScript config and every loaded extension by digest. The package graph and resolution options are also inputs because dependency-cruiser relies on enhanced-resolve and tsconfig-paths support.

### Stryker

StrykerJS 10.0.0's JSON reporter writes a `MutationTestResult` directly. The core package depends exactly on `mutation-testing-report-schema@3.8.4`, `mutation-testing-metrics@3.8.4`, and its own 10.0.0 API and instrumenter packages. [Stryker core manifest](https://raw.githubusercontent.com/stryker-mutator/stryker-js/master/packages/core/package.json), [JSON reporter source](https://raw.githubusercontent.com/stryker-mutator/stryker-js/master/packages/core/src/reporters/json-reporter.ts)

The Draft 7 report schema requires `schemaVersion`, `thresholds`, and `files`. File keys are relative paths; each file contains language, full source, and mutants. Each mutant contains an ID, mutator name, location, and status, with optional covered-by tests, killing tests, duration, replacement, status reason, and tests completed. Optional test-file definitions connect test IDs to names and locations. [`mutation-testing-report-schema@3.8.4`](https://raw.githubusercontent.com/stryker-mutator/mutation-testing-elements/v3.8.4/packages/report-schema/src/mutation-testing-report-schema.json)

The schema does not contain a `mutationScore` field. Section 9's example metric can still be supported, but the adapter must label it as derived and record the exact `mutation-testing-metrics@3.8.4` formula or Warrant-owned formula. A contract must not pretend it read that scalar directly from Stryker's JSON.

Mutation results are not deterministic evidence in the same sense as a parser result. Concurrency, test flakiness, timeouts, coverage, and host load can change statuses and durations. Warrant should bind the full raw report, runner and plugin versions, test command, config, concurrency, timeout policy, environment class, and snapshot. A surviving mutant is a result; a different duration is not a model change.

### Vitest

Vitest 5.0.1 intentionally emits JSON similar to Jest. The root contains suite and test counts, `startTime`, `success`, snapshot summary, `testResults`, and optional coverage. Each test-file result has `name` as the file path, file `startTime` and `endTime`, status, message, and `assertionResults`. Each assertion includes ancestor titles, full name, title, status, optional duration, optional line and column, failure messages, metadata, tags, and benchmarks. [`vitest` package manifest](https://raw.githubusercontent.com/vitest-dev/vitest/main/packages/vitest/package.json), [Vitest JSON reporter](https://raw.githubusercontent.com/vitest-dev/vitest/main/packages/vitest/src/node/reporters/json.ts)

Per-test duration is present when the runner records it. The file path is present at `testResults[].name`, not repeated on each assertion, so the adapter must inherit the enclosing file. Assertion location has line and column only. No formal JSON Schema was verified.

The report's timestamps and durations are intentionally variable, and test results can be flaky. Warrant should treat counts, outcomes, test identities, file identity, and failures as evidence bound to the run, while excluding timing fields from a semantic report comparison. Pin Vitest, Vite and environment packages from the project lock, Node, config digest, project selector, pool settings, arguments, and adapter.

### Jest

Jest 30.5.1 emits `FormattedTestResults` for `--json`. The root contains suite and test counts, snapshot summary, start time, success, interruption state, optional coverage, and `testResults`. A formatted file result contains `name` as the test file path, start and end times, status, message, summary, coverage, and assertion results. Assertions contain titles, full name, status, optional duration, optional line and column, and failure messages. [`jest` package manifest](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest/package.json), [test-result types](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-test-result/src/types.ts), [assertion-result type](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-types/src/TestResult.ts), [formatter](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-test-result/src/formatTestResults.ts)

Per-test durations are therefore present when recorded, and file paths are present on the enclosing test result. The optional assertion location does not repeat the file path. Jest serializes the formatted result after any configured test-results processor, so that processor's identity is also part of the instrument if present. [`runJest.ts`](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-core/src/runJest.ts)

Jest's own scheduler uses stable JSON serialization in some internal paths, but the report includes clocks, durations, failures, and potentially coverage. Warrant must not use raw byte equality as test determinism. Pin Jest, runner, environment, transformer, test-results processor, Node, config, arguments, shard and worker settings, and adapter.

### Binding reports to snapshots

None of Knip, Fallow, dependency-cruiser, Stryker, Vitest, or Jest natively places Warrant's Git tree identity in its ordinary report. An import command cannot retroactively prove which tree produced a file merely because the operator supplies `--snapshot` later.

Warrant should invoke or wrap instruments inside `warrant attest run`, capture the tree before and after, digest stdout and the report file, and then import the report through that receipt. For reports produced elsewhere, require a signed CI attestation that binds the report digest to the tree. Otherwise the evidence is `report-unbound`. This changes section 9 from "the report names the tree" to "the producing receipt binds the report digest to the tree."

## Recommendations for the spec

### Section 3, architecture

- confirmed: keep `oxc_parser`, `oxc_ast`, and `oxc_semantic` as the Rust-native syntax and per-file binding seam.
- confirmed: keep `oxc_resolver` behind a `tsc --traceResolution` qualification instead of calling it compiler-authoritative by inspection.
- confirmed: keep ast-grep as the embedded structural-pattern engine, with the three ast-grep crates exact-pinned together and the TypeScript grammar treated as an instrument input.
- changed: lead with an exact-version TypeScript 7 unstable async sidecar, with LSP as the S1 fallback.
- changed: remove `@typescript/api` package references and do not call the unstable API stable.
- changed: replace "JSON reporters for ... tsc" with "versioned adapters for TypeScript diagnostics and `--traceResolution` text." No current first-party JSON reporter for `tsc` was verified.
- changed: state that test and analyzer reports are bound to a snapshot by the producing Warrant receipt or external attestation, not by a tree field native to those formats.

### Section 6, program model

- confirmed: `symbol_level: binding` accurately describes the Oxc fast path. `oxc_semantic` supplies per-file scopes, symbols, and references; `lang-ts` must add module resolution and re-export traversal.
- changed: the example capability report should separate `compiler_reference_instrument: "typescript@7.0.2/unstable-async"` from `resolution_oracle: "typescript@7.0.2 --traceResolution"`. One `typescript` string currently hides two distinct claims.
- changed: list `node16`, `nodenext`, or `bundler` in `supports` only after S6 qualifies that mode.
- changed: split S1's SCIP candidate into the published 0.4.0/TypeScript 5 artifact and a deliberately source-pinned TypeScript 6 build.
- changed: treat those SCIP builds as different instruments.
- changed: add the TypeScript 7 unstable async API sidecar as the leading S1 candidate. Its output schema is Warrant-owned and versioned; TypeScript remote handles never cross into Rust.
- unresolved: compiler-level reference precision, whole-index memory, and batch duration remain spike results. Upstream compiler speed does not establish Warrant index speed.

### Section 9, evidence and instruments

- confirmed: exact versions, invocations, schema identities, config digests, and upgrade comparisons belong in `warrant/instruments.lock` and receipts.
- changed: extend npm lock entries with registry integrity, resolved dependency-lock digest, actual runtime version, platform-binary digest where present, output schema digest, and adapter version.
- changed: define raw report evidence separately from Warrant's canonical normalized rows. Store and sign the raw digest; use canonical rows for comparison and evaluation.
- changed: bind report digests through producing receipts or signed external attestations; otherwise a later raw import is unbound.
- changed: identify Stryker's `mutationScore` as a derived adapter metric. Pin the formula or `mutation-testing-metrics@3.8.4`; the JSON schema contains mutant statuses, not that scalar.
- changed: document successful finding exit codes per instrument. Fallow exit 1 means a successful run with findings, not a tool failure.
- unresolved: no formal JSON Schema was verified for Knip, Vitest, or Jest. Their pinned source interfaces and Warrant adapter fixtures define the qualification contract.

### Section 17.7, spikes

- changed: S1 compares the TypeScript 7 unstable async sidecar, LSP, and the exact published SCIP artifact.
- changed: S1 requires repeatable indexes, coverage, precision and recall, resource data, and explicit unsupported facts.
- confirmed: keep the existing 0.98 thresholds; TypeScript 5 or 6 results never establish TypeScript 7 authority.
- changed: S6 requires zero target or outcome disagreements for every enumerated edge; missing, extra, ambiguous, and unparsed edges fail.
- changed: S6 records a feature matrix and qualifies each project only for exercised modes.
- confirmed: a project with any unresolved S6 disagreement remains `unqualified` and makes analysis incomplete unless it falls back to compiler-produced resolution.

### Section 19, build-start facts

Every `[verify: ts-stack]` marker should resolve to these exact strings. Versions not directly named by a marker are included where the adapter or ABI requires them.

| Section 19 item | Exact version string | Disposition |
| --- | --- | --- |
| Oxc parser, AST, semantic, span, allocator set | `0.150.0` | confirmed |
| `oxc_resolver` | `11.24.3` | confirmed |
| ast-grep core, config, language set | `0.45.3` | confirmed |
| `tree-sitter` | `0.27.0` | confirmed |
| TypeScript grammar | `tree-sitter-typescript@0.23.2` | add to lock or Cargo resolution record |
| Stable native TypeScript compiler and parity oracle | `typescript@7.0.2` | changed from placeholder to exact pin |
| Last JavaScript compiler | `typescript@6.0.3` | record when actually resolved |
| TypeScript 6 compatibility wrapper | `@typescript/typescript6@6.0.2` | exact wrapper pin; record resolved 6.0.3 compiler separately |
| Last 5.x compatibility baseline | `typescript@5.9.3` | record only when a 5.x instrument runs |
| Legacy native preview | `@typescript/native-preview@7.0.0-dev.20260707.2` | current registry fact, do not select for new work |
| SCIP indexer | `@sourcegraph/scip-typescript@0.4.0` | candidate only; published artifact resolves `typescript@^5.6.2` |
| Rust SCIP reader | `scip@0.10.0` | add if SCIP candidate survives S1 |
| Knip | `knip@6.36.0` | initial evidence lock |
| Fallow | `fallow@3.27.0` | changed from the prior day's 3.26.0 lead |
| dependency-cruiser | `dependency-cruiser@18.3.1` | census importer lock |
| StrykerJS | `@stryker-mutator/core@10.0.0` | initial evidence lock |
| Stryker report schema and metrics | `mutation-testing-report-schema@3.8.4`, `mutation-testing-metrics@3.8.4` | add to adapter identity |
| Vitest | `vitest@5.0.1` | initial evidence lock |
| Jest | `jest@30.5.1` | initial evidence lock |

The section 19 TypeScript sentence should say that Node is absent from the Oxc fast path but required for the recommended compiler-authority sidecar, parity runs, and npm instruments. The native compiler executable itself comes from platform packages selected by `typescript@7.0.2`; receipts should record the installed platform package and binary digest.

## Verification notes

Research was performed on 2026-09-17. The report path retains the date requested in the brief. No package was installed, no repository was analyzed with these tools, and no performance or parity corpus was run.

### Repository material read

- `docs/vision.md` in full.
- sections 3, 6, 9, 17.7, and 19 of `docs/specs/2026-09-16-warrant-v1-spec.md`; section 6 was read in full.
- `docs/design/atlas-quality-research/2026-09-16-quality-oxc-fallow.md` and `docs/design/atlas-quality-research/2026-09-16-ts7-architecture-opportunities.md` as leads only.

### TypeScript primary sources read

- the [`typescript`](https://registry.npmjs.org/typescript), [`typescript@7.0.2`](https://registry.npmjs.org/typescript/7.0.2), [`@typescript/native-preview`](https://registry.npmjs.org/@typescript%2fnative-preview), [`@typescript/typescript6`](https://registry.npmjs.org/@typescript%2ftypescript6), and [`@typescript/api`](https://registry.npmjs.org/@typescript%2fapi) registry endpoints, including published package manifests and executable/export maps. These establish the npm state used in the version table;
- Microsoft's [TypeScript 7 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/), [TypeScript 6 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-6-0/), and [native preview announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-native-previews/);
- the typescript-go [README](https://raw.githubusercontent.com/microsoft/typescript-go/main/README.md), native-preview [package manifest](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/package.json), [LSP entrypoint](https://raw.githubusercontent.com/microsoft/typescript-go/main/cmd/tsgo/lsp.go), [API entrypoint](https://raw.githubusercontent.com/microsoft/typescript-go/main/cmd/tsgo/api.go), [LSP server registrations](https://raw.githubusercontent.com/microsoft/typescript-go/main/internal/lsp/server.go), async [API](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/async/api.ts), async [client](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/async/client.ts), generated [protocol](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/proto.generated.ts), and sync [channel](https://raw.githubusercontent.com/microsoft/typescript-go/main/_packages/native-preview/src/api/syncChannel.ts);
- the API scaffold [pull request](https://github.com/microsoft/typescript-go/pull/711), package consolidation [pull request](https://github.com/microsoft/typescript-go/pull/3430), and complex-extension API [design issue](https://github.com/microsoft/typescript-go/issues/2824);
- TypeScript's [`traceResolution`](https://www.typescriptlang.org/tsconfig/traceResolution.html) and [module reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html).

### SCIP primary sources read

- the `@sourcegraph/scip-typescript` [registry record](https://registry.npmjs.org/%40sourcegraph%2fscip-typescript/0.4.0), default-branch [package manifest](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/package.json), [README](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/README.md), [CLI options](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/src/CommandLineOptions.ts), and [indexer entrypoint](https://raw.githubusercontent.com/sourcegraph/scip-typescript/main/src/main.ts). These establish the published-versus-default-branch mismatch and project traversal;
- the [SCIP protobuf schema](https://raw.githubusercontent.com/sourcegraph/scip/main/scip.proto), `scip` [registry record](https://crates.io/api/v1/crates/scip), and 0.10.0 [crate archive](https://static.crates.io/crates/scip/scip-0.10.0.crate).

### Oxc and pattern-engine primary sources read

- crates.io records for [`oxc_parser`](https://crates.io/api/v1/crates/oxc_parser), [`oxc_ast`](https://crates.io/api/v1/crates/oxc_ast), [`oxc_semantic`](https://crates.io/api/v1/crates/oxc_semantic), [`oxc_span`](https://crates.io/api/v1/crates/oxc_span), [`oxc_allocator`](https://crates.io/api/v1/crates/oxc_allocator), and [`oxc_resolver`](https://crates.io/api/v1/crates/oxc_resolver). These establish the crate versions and MSRVs;
- the Oxc [workspace manifest](https://raw.githubusercontent.com/oxc-project/oxc/main/Cargo.toml), parser [changelog](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_parser/CHANGELOG.md), AST [changelog](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_ast/CHANGELOG.md), semantic [library](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/src/lib.rs), semantic [builder](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/src/builder.rs), and semantic [README](https://raw.githubusercontent.com/oxc-project/oxc/main/crates/oxc_semantic/README.md);
- the resolver [README](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/README.md), [manifest](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/Cargo.toml), [options](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/src/options.rs), [tsconfig model](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/src/tsconfig.rs), NAPI [types](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/napi/index.d.ts), and [changelog](https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/CHANGELOG.md);
- crates.io records for [`ast-grep-core`](https://crates.io/api/v1/crates/ast-grep-core), [`ast-grep-config`](https://crates.io/api/v1/crates/ast-grep-config), [`ast-grep-language`](https://crates.io/api/v1/crates/ast-grep-language), and [`tree-sitter`](https://crates.io/api/v1/crates/tree-sitter), plus the ast-grep [rule](https://ast-grep.github.io/reference/rule), [YAML](https://ast-grep.github.io/reference/yaml), [API](https://ast-grep.github.io/reference/api), and [language-binding](https://ast-grep.github.io/contributing/add-lang) documentation;
- Tree-sitter's [ABI table](https://tree-sitter.github.io/tree-sitter/using-parsers/7-abi-versions.html), the 0.27.0 [crate archive](https://static.crates.io/crates/tree-sitter/tree-sitter-0.27.0.crate), and tree-sitter-typescript's [Rust binding](https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/bindings/rust/lib.rs), [manifest](https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/Cargo.toml), [metadata](https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/tree-sitter.json), and generated TypeScript and TSX parsers;
- Semgrep's [pattern syntax](https://docs.semgrep.dev/writing-rules/pattern-syntax), [rule syntax](https://docs.semgrep.dev/writing-rules/rule-syntax), repository [README](https://raw.githubusercontent.com/semgrep/semgrep/develop/README.md), and [license](https://raw.githubusercontent.com/semgrep/semgrep/develop/LICENSE).

### Instrument primary sources read

- Knip's [registry record](https://registry.npmjs.org/knip), [package manifest](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/package.json), JSON [reporter](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/src/reporters/json.ts), and [issue types](https://raw.githubusercontent.com/webpro-nl/knip/main/packages/knip/src/types/issues.ts). These establish the pin and reporter fields;
- Fallow's [registry record](https://registry.npmjs.org/fallow/3.27.0), [README](https://raw.githubusercontent.com/fallow-rs/fallow/main/README.md), [workspace manifest](https://raw.githubusercontent.com/fallow-rs/fallow/main/Cargo.toml), and [output schema](https://raw.githubusercontent.com/fallow-rs/fallow/main/docs/output-schema.json);
- dependency-cruiser's [registry record](https://registry.npmjs.org/dependency-cruiser), [package manifest](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/package.json), [rules reference](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/doc/rules-reference.md), [configuration schema](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/schema/configuration.schema.json), JSON [reporter](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/report/json.mjs), and [result schema](https://raw.githubusercontent.com/sverweij/dependency-cruiser/main/src/schema/cruise-result.schema.json);
- Stryker's [registry record](https://registry.npmjs.org/%40stryker-mutator%2fcore), core [manifest](https://raw.githubusercontent.com/stryker-mutator/stryker-js/master/packages/core/package.json), JSON [reporter](https://raw.githubusercontent.com/stryker-mutator/stryker-js/master/packages/core/src/reporters/json-reporter.ts), and pinned 3.8.4 [report schema](https://raw.githubusercontent.com/stryker-mutator/mutation-testing-elements/v3.8.4/packages/report-schema/src/mutation-testing-report-schema.json);
- Vitest's [registry record](https://registry.npmjs.org/vitest), [package manifest](https://raw.githubusercontent.com/vitest-dev/vitest/main/packages/vitest/package.json), and JSON [reporter](https://raw.githubusercontent.com/vitest-dev/vitest/main/packages/vitest/src/node/reporters/json.ts);
- Jest's [registry record](https://registry.npmjs.org/jest), [package manifest](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest/package.json), formatted-result [types](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-test-result/src/types.ts), assertion [types](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-types/src/TestResult.ts), [formatter](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-test-result/src/formatTestResults.ts), and JSON write path in [`runJest.ts`](https://raw.githubusercontent.com/jestjs/jest/main/packages/jest-core/src/runJest.ts).

### Could not verify

- a supported stable TypeScript 7 compiler API in 7.0.2, a published `@typescript/api` package, or a versioned stability promise for the raw API RPC protocol;
- a standard LSP request that returns the complete TypeScript `Program` or every module-resolution result in one batch;
- a published `scip-typescript` artifact containing the default branch's `typescript@^6.0.3` dependency;
- complete current parity of `oxc_resolver` with TypeScript 7 for `bundler`, `node16`, or `nodenext`, or a maintained upstream list of all remaining divergences;
- a formal JSON Schema for Knip, Vitest, or Jest, or a canonical output-order promise for Knip and dependency-cruiser;
- deterministic mutation or test outcomes for Stryker, Vitest, or Jest;
- Warrant-specific time and memory results for any of the four program-model routes.
