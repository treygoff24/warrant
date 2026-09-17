<!-- Provenance: copied 2026-09-16 from the Atlas repository (docs/plans/rebuild-research/ts7-upgrade/architecture-opportunities.md), where Astra wrote it as pre-M2 quality-tooling research for Atlas. It is the TypeScript 7 upgrade architecture scout. Copied into Warrant unchanged except that local filesystem paths were generalized to ~/ form. The Atlas-specific rulings inside apply to Atlas, not to Warrant; Warrant's spec cites this file for the tool survey and the reasoning. -->

# Atlas TS7: architecture opportunities

## Coordinator adjudication, 2026-09-16

This is independent architecture advice; the final plan governs proposed implementation. The later [compatibility scout](atlas-compatibility.md#a-narrowly-scoped-ts6-api-dependency) and the parent's pinned-source read establish that [Next16.3.4's published resolver](https://unpkg.com/next@16.3.4/dist/lib/typescript/runTypeScriptCli.js) resolves `typescript/package.json`, not shell PATH. This is source-inferred, not runtime-proven.

The generic Microsoft root TS6 alias recommendation must not be adopted blindly: Next could select TS6 even while the shell's `tsc` is native TS7. Prefer a real root TS7 package with isolated API consumers; actual npm resolution and Next build proof are mandatory. Earlier alias advice below is retained as superseded where relevant.

Recommend a qualified stable TS7 compiler cutover, followed by schema-to-handler inference and a narrow native typed-lint lane. Keep Atlas's business architecture. Do not use this upgrade to redesign DBOS, database access, transports, or the quality factory.

## Evidence and authority

Inspected HEAD: `2b4fbb9235ce795f456bb700291f0c288caac6b5`, obtained with `git rev-parse HEAD`. Source references below describe working-tree reads during this pass, not a verified clean checkout. Concurrent documentation changes are possible.

Governing context: supplied `AGENTS.md`; spec `docs/specs/2026-09-02-atlasos-rebuild-spec.md:126-145` (§1.5), `744-758` (§18), `806-821` (§21); latest `docs/plans/live-state.md:1-5`. The September 16 TS7 ruling approves planning scope, not implementation. The historical spec TS5.9 pin does not override that ruling.

Prior leads: `docs/plans/rebuild-research/2026-09-16-atlas-quality-factory.md:61-77`; `2026-09-07-pin-recon.md:50-59`. Both recommended investigating compiler-API consumers. Neither proves present compatibility.

Fresh upstream reads: [npm latest metadata](https://registry.npmjs.org/typescript/latest) identifies 7.0.2 artifacts; [Microsoft's July 8 release announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/) documents native checking, watch, defaults, parallelism and side-by-side TS6 tooling. This is published evidence, not installed or executed evidence.

Important source nuance: the announcement says 7.0 ships no API; current registry metadata exposes `./unstable/*` paths. Therefore use the narrower claim **no established drop-in legacy JavaScript compiler API compatibility**. Unstable exports do not establish support for Atlas's generators or ESLint. Reconcile exact package/API status during qualification, not by copying the old note's absolute wording.

During the original architecture pass, `codebase-design` loaded. The `skill://write-human` lookup failed, so that lane read Atlas's checked-in `packages/core/src/skills/write-human.md` instead. The `skill://exa-agent` lookup also failed; the lane fell back to direct primary-source URL reads and available research guidance. It did not use Exa. No agents, installs, application gates, services, secrets, Beads, commits, or dependency/config edits were performed.

For this September 16 prose correction, the real `~/.codex/skills/write-human/SKILL.md` and its audit reference were loaded, and its `scan.py` was run on this document.

The original work was the single independent architecture pass for the parent's Level2 plan. It neither read nor awaited the upstream-delta and atlas-compatibility reports. The parent is the coordinator; Union Alpha was the architecture/research workhorse. The parent owns evidence reconciliation, the visual explainer, the one human implementation approval, and the planned fresh adversarial review. This advice creates no extra approval round. No model-performance or subscription-lane claim is made.


## Before and after

| Surface | Observed before | Recommended after |
| --- | --- | --- |
| Type authority | Root `tsc -p tsconfig.json`, TS5.9.3; `package.json:20,78` | Exact qualified stable TS7 executable; standalone typecheck remains authoritative |
| Program shape | One root no-emit program covering core/apps/tests; no project references; `tsconfig.json:1-17` | Same source reachability and imports, explicit environment globals; no reference-build conversion merely to use `--builders` |
| Execution | Node24 executes worker TS directly; Next builds web; `deploy/Dockerfile.worker:23-29,71` and `apps/web/package.json:6-8` | Same runtime/tool split; compiler speed is development feedback, not request/DBOS speed |
| Tool authoring | `ToolModule.definitions` contextualizes handlers as `ToolDefinition<any, any>`; `contracts/tools.ts:15-46` | Infer each definition from its schemas before collection erases heterogeneous types |
| Lint | Syntax-oriented recommended ESLint plus four Atlas rules; `eslint.config.js:5-28` | Qualified native typed rules plus retained custom/gap ESLint; explicit non-overlapping ownership |
| Safety evidence | Root compiler, integrated gate, separate crash/browser commands | Preserve all existing meaningful tests and the full gate; add targeted negative type/lint proof, not a replacement suite |

All abbreviated core paths below are relative to `packages/core/src/` unless explicitly rooted.

The deep module remains `actions`: callers submit a definition/input and context; authorization, approval consumption, lock ordering and exact-once writes stay inside. The registry stays a deterministic composition of definitions, not a framework or generated router. Three adapters continue projecting one implementation. The worker owns durable execution and serialization, with no runtime SDK imported into core.

### Invariants that must survive

Tool names, active inventory, schemas on the wire, result envelopes, idempotency identity, authorization and approval semantics remain unchanged. `packages/core/test/tools/registry-transports.test.ts:1-5,76-104` names the independent 33-tool inventory; do not replace it with a self-derived expectation.

`actions/dispatcher.ts:58-69` validates unknown input before the resource/execute path; stronger static inference must not remove runtime parsing. MCP advertises the complete result envelope, not bare tool output (`tools/to-mcp.ts:35-55`); preserve its negative fixture (`packages/core/test/tools/to-mcp.test.ts:117-125`).

Tenant schema branding and the allowlisted context construction remain (`contracts/ctx.ts:7-18`, `db/context.ts:7-19`). No new per-call tenant predicates or alternate transaction interface.

Preserve Node24, ESM/NodeNext, ES2023 target, `noEmit`, `erasableSyntaxOnly`, `.ts` import extensions and `verbatimModuleSyntax` (`tsconfig.base.json:3-19`). Native compilation does not authorize changing runtime syntax, Node APIs or deployment topology.

Preserve workflow names/checkpoint ordering and the serializer in `apps/worker/tools.ts:12-26,43-65`. Its settled internal chain deliberately permits the next call after failure; the returned `next` still delivers failure to the caller. A broad "no swallowed rejections" cleanup could break it.


## Prioritized opportunities

### P1: Type each tool before heterogeneous collection

**Observed:** `contracts/tools.ts:35-40` explicitly defines `AnyToolDefinition = ToolDefinition<any, any>`. `graph/tools.ts:47-64` assigns the whole module to `ToolModule`; `execute(input, ...)` therefore does not gain its input shape from the adjacent Zod schema. `campaign/tools.ts:109-120` already demonstrates local `z.infer`, but the module is widened again at `122-135`. This is a source-level inference gap, not an executed compiler result.

**Change:** preserve the existing `ToolDefinition<I,O>` interface and add the smallest definition-authoring seam that binds input/output schemas before inserting the definition into `ToolModule`. Infer the handler input from the parsed schema output (not raw `z.input`, which may allow defaults/transforms); bind execute return, event result and enqueue result to the declared output. Prefer one identity-style generic constructor over repeated handwritten handler types. It must not allocate per call or add runtime dispatch/parsing. Prove generic inference before selecting its exact signature. Keep heterogeneous erasure local; replacing `any` with `unknown` everywhere is not variance-safe and does not itself restore correlation.

**Concrete proof:** on `search_people`, planting `input.qurey`, passing a string `limit`, or returning an output incompatible with its schema must fail native checking. A valid optional input and a separate default/transform fixture must compile and execute through the existing dispatcher. Repeat the authoring conversion for every registered definition and affected fixtures; no partially converted registry. Keep registry transport/inventory tests and approval/undo behavior tests.

**Value/risk:** highest leverage because one authoring rule protects every tool without changing consumers. Moderate cross-module diff and high review sensitivity at the actions seam. TS5.9 could already express this; TS7 provides cheaper checking, not automatic repair. No claim that `satisfies ToolModule` alone fixes contextual `any`.

### P2: Spend native checking on meaningful typed lint

**Observed:** ESLint configuration enables no type-aware project service (`eslint.config.js:5-28`). `apps/worker/main.ts:371-374` discards a shutdown promise chain without a rejection branch; `apps/worker/health.ts:54-57` does likewise for its response callback. `health` catches database failures (`24-44`), so these are qualification examples, not established production failures. `apps/worker/tools.ts:20-26` is a required clean counterexample.

**Change:** qualify `no-floating-promises`, `no-misused-promises`, selected unsafe-flow rules and switch exhaustiveness with exact native engine/rule versions. The [Oxlint guide](https://oxc.rs/docs/guide/usage/linter/type-aware) says TS7+ is required and uses a separate `oxlint-tsgolint` engine. Configure floating-promise detection so `void` is not assumed to handle rejection; test the exact chosen rule options. Catch handlers must implement an intentional failure contract, not silently consume errors to satisfy lint. Do not change shutdown or health semantics without that contract and behavior proof.

**Proof:** a discarded rejecting promise must fail lint; an awaited promise and the serialization pattern must pass. Include an unsafe schema-to-handler value flow from P1, not only synthetic assignments. All four Atlas custom rules retain positive/negative fixtures. Record unmatched files and resolved programs: traversal is not evidence every file received type information. Keep separate native `tsc`; combined Oxlint checking is not yet evidence of equal coverage.

**Value/risk:** catches concrete async/unsafe-value mistakes at development time. Engine/compiler skew, default rule exemptions and resource consumption remain risks. These rule categories predate TS7; the native route is the unlock. No Oxfmt/Fallow/Specgate or broad lint-policy migration belongs in this task. The retained ESLint lane embeds the legacy JavaScript compiler API through `typescript-eslint`. The original recommendation to qualify the announced TS6 compatibility package (`@typescript/typescript6`) remains a consumer-isolation lead; any reading that prescribes a root TS6 alias is superseded by the coordinator adjudication above. Keep the root genuinely TS7 and prove retained ESLint resolves an API-bearing compiler in isolation.

### P3: Make compiler reachability explicit and preserve generated typing

**Observed:** root includes only package/app TS/TSX (`tsconfig.json:6-16`), omitting root `scripts/*.ts` and root TS configs as entrypoints. Tests may import some scripts, but that does not prove complete reachability. No `types` allowlist is present. `contracts/contracts.test-d.ts` is a `.ts` file with compile-time equality assertions; do not assume it is a Vitest runtime test or an excluded `.d.ts`. `vitest.config.ts:6-15` collects different runtime globs.

**Change:** capture actual resolved file/config inventories first. Make required ambient globals explicit under TS7's `types: []` default, without blindly adding every installed `@types` package. Preserve imported generated declarations even where generated directories are excluded as entrypoints. Cover first-party tooling TS through a small explicit check scope only if the inventory proves it missing; do not broaden to all JS or create a monorepo build graph.

**Workaround check:** `apps/web/panda.config.ts:13-23` intentionally emits `.js` alongside `.d.ts` to satisfy NodeNext. Keep it. A native compiler is not evidence that NodeNext's `.mjs` declaration pairing changed. Root `prepare` runs Panda (`package.json:17-19`); worker production install intentionally skips it (`deploy/Dockerfile.worker:23-27`). `schema/auth.generated.ts:1-35` is a schema-qualified auth output, with a targeted existing lint exception. Do not hand-edit or suppress generated types to make TS7 green.

**Proof:** negative type errors in core, web TSX, worker, CLI, contract tests and tooling must all reach their intended checks; valid generated imports must have concrete types rather than implicit `any`. Compare owned generator outputs and semantics in disposable copies after authorization. Better Auth/Drizzle regeneration requires a qualified, version-pinned command and authorized scope; the uninstalled Better Auth CLI is not a compulsory regeneration prerequisite for compiler cutover. Preserve declarations and schema semantics; no migration SQL from a compiler upgrade.

**Value/risk:** avoids a fast but incomplete gate and catches global/config migration errors early. TS7 changes defaults and implementation behavior; full inventory and generator compatibility remain unrun.

### P4: Evaluate existing strict flags separately; do not make them a hidden prerequisite

Recommend **diagnostic evaluation now in the approved migration execution, enforcement deferred from the mandatory compiler cutover** for both `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`. Do not silently enable either. They are intentionally selected older improvements, not TS7 inventions. Report diagnostics by production/test/generated ownership and distinct remediation shape, with exact file counts and reviewed examples; this pass cannot supply that size without executing a compiler.

**Indexed access:** `graph/writes.ts:51,97,128` and `campaign/campaigns.ts:77` assert first rows exist, while `actions/dispatcher.ts:51-55,84-87` already demonstrates checked row extraction. `noUncheckedIndexedAccess` exposes *unasserted* lookups; it does not defeat existing `!`. Review insertion guarantees versus conflict/delete races before choosing an invariant guard or typed tuple. Do not rewrite all SQL or add a global "first row" wrapper. Existing guarded lookup is the preferred local pattern.

**Optional properties:** `contracts/ctx.ts:10-16` declares omission; `db/context.ts:18-19` already omits undefined values. In contrast `graph/tools.ts:64` and `campaign/tools.ts:135,159-162` pass optional values as present keys. `graph/writes.ts:35-38` explicitly defines undefined as leave-alone and null as clear; `campaign/campaigns.ts:80-100` describes partial updates. Preserve these semantics. Where explicit undefined is genuinely accepted, `?: T | undefined` is an honest contract; adding it everywhere to silence errors defeats the flag.

Promotion is coherent only if every affected caller/generated interface can be corrected under one reviewed contract, no mass assertions/unions/suppression are needed, and omission/undefined/null behavior has proof. A broad patch, library mismatch or runtime policy ambiguity means stop that optional substep and report the measured scope. This does not block reporting a compiler-only result, but the parent must label stricter-policy work deferred rather than "fully adopted."


### P5: Follow up on real trust seams, not a type-count contest

`apps/cli/src/client.ts:58,103-110,115-122` casts parsed JSON to records/results; `decide` checks only the status discriminant before a double assertion. `apps/cli/src/login.ts:103` and `token-store.ts:83` contain similar assertions; `gmail/google-port.ts:256,307` casts provider responses. Static upgrading cannot validate network/storage bytes. A shared runtime result validator could remove the CLI cast and reuse the MCP result vocabulary, but rejecting malformed successful responses changes runtime behavior. Defer that concrete hardening to a separately specified task unless native lint makes it a mandatory, bounded finding. Do not hide it with an unsafe-rule exclusion.

Manual types are not all duplicates: `campaign/campaigns.ts:11-35` separates snake_case database rows/timestamps from camelCase serialized domain fields; replacing both with `$inferSelect` would conflate contracts. `tools/to-cli.ts:75-95` models the JSON Schema subset its projection consumes, not a second tool registry. `contracts/contracts.test-d.ts:22-89,130-142` deliberately repeats contract shapes; retain meaningful authority/brand/transport constraints. If the authoring seam changes, replace only assertions that freeze its old erasure mechanism with consumer rejection/acceptance tests; do not mechanically re-pin or delete the entire contract file.

## Adopt / reject / defer

| Opportunity | Recommendation | Reason | Size/proof gate |
| --- | --- | --- | --- |
| TS7 compiler cutover, exact stable pin | **Adopt** (gated) | Ruled sprint scope; native speed is real development feedback; pinned Next16.3.4 source supports CLI checking through the resolved `typescript` package, subject to npm and Next execution proof | Full integrated npm gate and meaningful existing tests on the same tree, before/after measurements |
| P1 schema-to-handler inference | **Adopt** | Closes real contract gaps; strongest typed contract gain | No partially converted registry; all definitions converted with dispatcher/registry tests green |
| P2 narrow native typed lint | **Adopt** (gated) | Native engine requires TS7; catches async/unsafe-flow defects early | Negative/positive rule fixtures; recorded unmatched files; compiler-side coverage unchanged |
| P3 explicit reachability/globals | **Adopt** (gated) | Default config changes could silently narrow checking | Resolved-file inventory before/after; typecheck negative in each workspace |
| P4 `noUncheckedIndexedAccess` | **Defer** beyond cutover | Evidence-based but policy-ambiguous, cross-cutting; diagnostic-only now | Diagnostic report with exact counts and reviewed examples; promote only under P4 promotion coherence test |
| P4 `exactOptionalPropertyTypes` | **Defer** beyond cutover | Same; interacts with optional-field semantics across schemas/UI/ports | Same measured-scope rule; document explicit-undefined choices where they are real contracts |
| P5 CLI/Gmail runtime validators | **Defer** to separately specified hardening | Runtime behavior change; needs own contract | One reviewed task with positive/negative malformed-response fixtures |
| Compiler-API consumers (`typescript-eslint`) | **Qualify isolated API dependency if required** | Legacy API consumers need an API-bearing compiler; generic root TS6 alias prescription is superseded | Real root TS7; exact npm consumer-resolution proof, native root `tsc`, Next checker and retained lint all work |
| Project references/`--builders` conversion | **Reject for now** | Single noEmit program is coherent; benefit unproven for Atlas's size | Only if measured typecheck cost justifies it; never during cutover |
| `--checkers` tuning | **Reject for now** (default 4) | Publication says varying it can surface order-dependent results | Revisit only with fixed checkers documented across environments |
| DBOS/core/business redesign | **Reject** | No migration evidence warrants revisiting the governing seams | Out of scope; no claim that static inspection proves every implementation correct |
| Decorator/runtime redesign | **Reject** | Prior pin recon flagged an emit issue, but its present status was not verified here | Treat prior issue as a qualification lead, not a fresh confirmed defect or reason to prescribe TS7.1 |
| Mass `.js`/generated-code migration | **Reject** | Panda `.js` output is an intentional NodeNext pairing workaround | Keep `apps/web/panda.config.ts:13-23`; do not chase cosmetic `.mjs` changes |

## Minimal coherent migration sequence: five tasks

1. **Qualify exact tool resolution before cutover.** After human authorization, capture the current gate and compiler file inventory in disposable copies. Verify the latest stable pin (currently 7.0.2), then qualify Next, Panda, Vitest and retained ESLint against it. Qualify Better Auth/Drizzle regeneration only where authorized and with a known version-pinned command. The original generic alias recommendation is superseded: Microsoft illustrates `typescript` resolving to `@typescript/typescript6` and a separate native alias owning `tsc`, but Next16.3.4 resolves `typescript/package.json` rather than shell PATH. Prefer real root TS7 plus isolated legacy API consumers. A differently named TS6 sidecar alone does not redirect imports or peers. Prove actual npm placement and both root and Next native checker selection. Record executable paths, package identities, lockfile/native platform dependencies and editor/LSP selection. Do not commit a failing intermediate cutover.
2. **Cut over compiler and explicit reachability.** Apply the qualified exact dependency arrangement and minimal config changes, preserving current strictness and runtime syntax. Include explicit globals, side-effect-import declarations where required, and only proven missing tooling entrypoints. Classify all changed diagnostics; repair source or stop at an incompatibility. Preserve the full `npm run gate`, no renamed bypass. Keep generator changes isolated and justified.
3. **Convert tool authoring at the existing seam (P1).** Prove schema-driven inference with the positive/negative fixtures, then convert every definition and affected caller/test in one coherent change. Trace implementation exports as well as the declaration-only contract file. Retain three-transport, envelope, ledger, approval and undo behavior evidence. Do not invent a typed router or change the heterogeneous dispatch contract to satisfy an aesthetic ban on `any`.
4. **Qualify and enable the narrow native typed-lint policy (P2).** Pin both engines, prove rule behavior including `void` and serializer counterexamples, record actual typed file coverage, retain custom/gap ESLint and standalone TS7 checking. Repair adjudicated findings with behavior proof; a native incompatibility is a stop for this deliverable, not silent permission to substitute a broad new linter stack.
5. **Verify the integrated result and report optional-policy scope.** Run the complete gate on the final candidate plus the existing dedicated crash and browser E2E commands. Record compiler/build/watch/RSS measurements and separate one-flag-at-a-time P4 diagnostic reports. Report P5 as deferred runtime hardening. Reconcile affected developer/plan references and remove temporary plants. No enforcement of P4 or P5 implementation is implied.

Oxfmt/Fallow/Specgate/deslop remain in the separate quality-factory scope. Retained ESLint compatibility and native typed-lint qualification are explicitly inside these five tasks. Execution may not replace a failure with mass assertions/skips/shims, automatic return to 5.9, or unapproved scope expansion.

### Practical stop conditions (tooling proves incompatible; no automatic revert)

- A required generator/auth/test tool fails TS7 or its CLI path in a way a side-by-side TS6 dependency cannot cover, with no bounded workaround inside the existing pin discipline.
- Native typed lint cannot analyze required files, silently drops a required rule, or disagrees with the authoritative compiler on an unresolved mandatory diagnostic.

  Keep existing protections operational, report the blocked row, and request a bounded plan amendment rather than declaring full adoption.
- Unresolved type errors require mass `any`/assertions/skips/config suppressions to reach green.
- The integrated npm gate cannot run meaningfully under the qualified pin after the documented attempts above; report the exact failing stage/versions rather than reverting.

## Performance methodology

Use paired disposable copies of one source snapshot, with identical generated inputs, application dependency pins, Node/npm, environment and CPU/memory limits. Record the complete lockfile/tool delta; compiler-specific platform packages and a required API compatibility dependency are controlled differences, not literally identical dependency sets. Measure compiler-only comparison separately from later inference/lint changes. Use an alternating order, one warm-up and at least five measured pairs; report raw values, median and range. No timed run may install packages or compete with other agent gates.

**Cold check:** fresh compiler process and no incremental/build-info state; state explicitly whether OS page cache is warm. Do not drop host caches on the shared machine. **Warm check:** repeat on the same files/cache policy; root checking currently has no incremental setting, so do not mislabel process restart as an incremental compiler result.

**Build:** cold and warm `npm run build --workspace apps/web`, with an identical `.next` cache lifecycle. Report total time and the typecheck portion if exposed. Atlas has no `tsc --build` graph to benchmark; do not create one just to obtain a TS7 builders result.

**Watch:** initial-ready latency, then save-to-diagnostic latency for a leaf edit and a shared contract edit, followed by fix/clear latency. Count stale/missed diagnostics. Include one Unicode edit for LSP source-range checking; no claim of a useful first-party template-literal Unicode inference migration was established.

**Resources:** wall/user/system time, peak RSS of the compiler and aggregate child-process memory for builds, CPU quota, idle watch RSS and memory limit. Keep the same native `--checkers` setting (start with documented default 4); measure single-threaded separately if attributing native versus parallel effects. Do not compare a one-core baseline against unrestricted TS7 without labeling the difference.

**Lint/gate:** measure standalone checking, typed lint and full gate separately. Type-aware lint's profiling mode adds overhead; disable it for comparative end-to-end runs. A slow Postgres/browser stage can dominate total gate time even if compilation improves. Report regressions or inconclusive variance; promise no speedup multiplier or application-runtime improvement.


## Exact acceptance rows (all unrun in this planning pass)

| ID | Required result | Evidence / failure condition |
| --- | --- | --- |
| A1 | Exact stable native TS7 is the local CLI and Next build checker; retained tools resolve their intended API package | Actual npm install/resolution and Next build proof are mandatory: versions, executable/resolve paths, lockfile and native platform package record. Wrong binary or automatic download fails. The [pinned Next16.3.4 resolver](https://unpkg.com/next@16.3.4/dist/lib/typescript/runTypeScriptCli.js) reads `typescript/package.json`, not shell PATH; source inference is not runtime proof. Current [Next docs](https://nextjs.org/docs/app/api-reference/config/typescript) are versioned 16.3.5, not proof of the pinned build. |
| A2 | No existing meaningful checking coverage lost; every new diagnostic classified | Resolved files/config diff and classified diagnostic changes; real negatives in core, TSX, worker, CLI, tooling and `contracts.test-d.ts` fail their intended check. Literal TS5.9/TS7 diagnostic equality, including counts, wording and order, is not required. New errors must be resolved or reported as incompatibilities, not suppressed to force green. |
| A3 | Full `npm run gate` exits zero on the final integrated candidate | Every stage in `scripts/gate.sh:23-37`: typecheck, lint, budget, tooling/core/alternate-schema/web/workflow tests, MCP conformance and web build. Preserve expected suites/counts with justified test changes; no zero-collection substitute. |
| A4 | Existing dedicated crash and browser E2E evidence remains meaningful | `npm run test:crash` and `npm run test:web:e2e` (`package.json:30-34`) run on the same candidate. Fixtures only; no live send or deployment implied. These are separate from the existing main gate, not falsely claimed already included. |
| A5 | All definitions check schemas against handlers before erasure | P1 typo, wrong argument and wrong output plants fail; valid optional/default/transform fixtures pass. Independent 33-name inventory and three-adapter behavior remain intact. |
| A6 | Native typed lint catches agreed defects without suppressing valid workflow behavior | Exact rule IDs/options; floating rejection fails even with `void`; handled promise and serializer pass. Four existing custom-rule protections retain meaningful positive/negative evidence; unexpected unmatched files fail. |
| A7 | Owned generator outputs and generated typing remain semantically stable without schema/runtime drift | Compare owned Panda/Next outputs and any authorized, qualified auth/Drizzle regeneration for output and semantic stability. Compulsory byte identity is superseded; account for generator metadata differences through review. Better Auth CLI is uninstalled/unlocked, and auth/Drizzle regeneration needs a qualified version-pinned command before it is runnable. An unavailable regeneration path is a reported prerequisite, not a fabricated pass or an automatic requirement to install a new tool. Preserve declarations and schema semantics; no handwritten generated fixes, new migration SQL or implicit-any generated imports. |
| A8 | No workaround inflation or changed business behavior to force green | Review assertions, `any`, suppressions, skipped tests, exclusions and error-policy changes. Raw grep counts cannot prove safety. Legitimate negative TYPE TESTS may use scoped `@ts-expect-error` with unused-directive failure; application suppressions that hide regressions are not allowed. No mass assertions, blanket suppressions or shims. |
| A9 | Performance and old-flag scope are measured rather than assumed | Controlled cold/warm check/build/watch/RSS report and raw paired results. Separate counts/examples for each P4 flag; enforcement remains deferred and explicit. |

### Required no-claim sentences

"No TS7 compiler, build, lint, generator or application test was executed in this architecture pass." "The published stable version is 7.0.2; Atlas compatibility and performance remain unproven." "Native compilation changes development tooling, not DBOS semantics or the speed of running JavaScript." "Schema inference improvements proposed here were expressible before TS7." "Neither `noUncheckedIndexedAccess` nor `exactOptionalPropertyTypes` is a TS7 invention or an automatic consequence of `strict`." "No inspected workaround is proven removable merely because the compiler is native." "No compile-time annotation validates network JSON or database bytes."

### Remaining unknowns and source status

No exhaustive AST count of casts/any/non-null assertions, compiler diagnostic inventory, lockfile-resolution qualification or inferred-handler experiment was run. The path examples are a bounded source inventory, not a security audit or a whole-repo zero-escape claim. Literal SQL `any(...)`, comments and Zod's `unrepresentable: "any"` are not TypeScript `any`; lexical hit counts must not be sold as typed debt. Generated-code determinism, exact legacy-consumer requirements, editor integration and strict-flag repair size remain execution questions.

Primary sources freshly read: [Microsoft TS7 release](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/), [npm latest](https://registry.npmjs.org/typescript/latest), [native change inventory](https://github.com/microsoft/typescript-go/blob/main/CHANGES.md), [Oxlint type-aware guide](https://oxc.rs/docs/guide/usage/linter/type-aware), [Next TypeScript guide](https://nextjs.org/docs/app/api-reference/config/typescript), [indexed access option](https://www.typescriptlang.org/tsconfig/noUncheckedIndexedAccess.html), [optional-property option](https://www.typescriptlang.org/tsconfig/exactOptionalPropertyTypes.html). The change inventory tracks upstream `main`, not the exact stable tag: its skipLibCheck conflict-reporting and JS/JSDoc changes are qualification leads, not asserted Atlas regressions. No inspected first-party JS/JSDoc or Unicode type utility justified a rewrite; keep existing skipLibCheck policy rather than widening suppression.
