<!-- Provenance: copied 2026-09-16 from the Atlas repository (docs/plans/rebuild-research/2026-09-16-atlas-quality-factory.md), where Astra wrote it as pre-M2 quality-tooling research for Atlas. It is the main report; the others are its scouts. Copied into Warrant unchanged except that local filesystem paths were generalized to ~/ form. The Atlas-specific rulings inside apply to Atlas, not to Warrant; Warrant's spec cites this file for the tool survey and the reasoning. -->

# A quality gate for Atlas before M2

Research checked September 16, 2026. Atlas source inspected at `24757c6`. Recommendations for Trey; no tooling installation, cleanup, application test run, configuration change, or M2 implementation is authorized by this report.

Scope ruling, September 16 at 23:42 UTC: Trey requires TypeScript 7+ migration in the pre-M2 re-baseline sprint. This supersedes the initial suggestion to defer the compiler migration. It authorizes inclusion in the sprint's scope, not installation or execution; the rest of the setup is still for discussion. The current stable npm release was rechecked as 7.0.2. Use a qualified exact stable pin, not an open-ended `>=7` dependency range.

## Recommendation

Put a quality phase before M2. Install and qualify the controls we choose, use them to rehabilitate the existing code without changing its intended behavior, then require the same controls throughout the build. This is a good experiment for Atlas and a reusable approach for the software factory.

I recommend Oxfmt, Oxlint, a deliberately narrow retained ESLint lane, Fallow, your Specgate, your deslop workflow, stronger Vitest coverage, fast-check, focused Stryker mutation tests, and local security and dependency checks. Keep the existing Postgres, DBOS, browser, and exact-contract tests. Give each check one job and one authoritative place in the gate.

The target should be **a verified clean baseline under an explicit quality policy**, not a score of 100 or a claim that code is objectively perfect. A program can have small functions, no duplicate tokens, and complete line coverage while implementing the wrong idea. Elegance also requires judgment about whether a concept needs to exist. We should make every mechanically enforceable requirement hard to evade, then put independent review around the remaining judgments.

The highest-value change is making the combined gate authoritative. Format-on-edit hooks help an agent correct itself. They cannot be the only thing preventing a bad change from entering the accepted build.

## What Atlas has today

These are observations from configuration and source, not fresh passing test results.

| Present control | What it already does | Gap to address |
| --- | --- | --- |
| TypeScript 5.9.3 with `strict: true` | Checks types across the four workspaces | No `noUncheckedIndexedAccess` or `exactOptionalPropertyTypes`; lint is not type-aware |
| ESLint and four tested Atlas rules | Protects selected database, schema, core/DBOS, and workflow boundaries | Limited syntax checks, not a complete import graph or runtime-safety proof |
| Main `npm run gate` | Typecheck, lint, budgets, tooling/core/web/workflow tests, MCP auth conformance, web build | No formatter, coverage, whole-project health check, dedicated crash suite, or browser E2E in this command |
| Separate acceptance runner | Includes the deliberate-defect crash suite | Preserve this stronger evidence; make its relationship to integration explicit |
| Vitest projects | Core tests under two schemas, web tests, Postgres workflow/crash tests | Most projects allow zero tests; no coverage configuration |
| Browser E2E and existing UI | Login, invitations, settings, CRM, approvals, and other user paths | Keep functional and accessibility checks even though we are skipping Impeccable and visual-style policing |
| Beads-managed Git hooks | Work-ledger integration | Not a code-quality gate; do not replace their hook path casually |
| Checked-in CI | `.github/workflows/ci.yml`, manual `workflow_dispatch` only | No automatic Forgejo workflow found in this tree; remote protections/runners were not inspected |
| Source/table/tool budgets | Existing architectural alarms | M2 proposes changing the counting interpretation; that remains a G1 decision, not something a cleanup quietly changes |

Sources: `package.json`, `tsconfig.base.json`, `eslint.config.js`, `eslint-rules/`, `vitest.config.ts`, `scripts/gate.sh`, `tests/acceptance.d/03-crash.sh`, `.github/workflows/ci.yml`, `.codex/hooks.json`, `.claude/settings.json`, and `.beads/hooks/`. Main-checkout dependencies were not installed for this research. No current-green claim is made.

One particularly good existing control deserves emphasis: the crash harness compares deliberately defective implementations with the intended implementation and checks that assertions distinguish them. That is stronger than simply adding more passing tests. We should extend it, not replace it with a fashionable tool.

## What the research supports

OpenAI's first-party account of agent-built software describes mechanically enforced dependency boundaries, custom lint rules, and recurring cleanup. The useful lesson is to encode the repository's actual architectural decisions as feedback the agent can act on. Its particular layer diagram and productivity claims are not a template or a guaranteed result for Atlas. [OpenAI engineering account](https://openai.com/index/harness-engineering/).

Anthropic's evaluation guidance distinguishes deterministic checks, model judgment, and the final state of the system. It also warns that shared environment state and infrastructure noise can distort comparisons. For this experiment, we should measure whether the controls catch defects and improve subsequent changes, not whether an agent reports success more confidently. [Agent evaluation guidance](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents), [infrastructure-noise study](https://www.anthropic.com/engineering/infrastructure-noise).

The SlopCodeBench authors study repeated extensions to an agent's own code. They report that quality guidance can improve initial structure without stopping later erosion. This is evidence for examining successive changes, not proof that a particular linter bundle solves the problem. Their benchmark is not Atlas, and its structural metrics do not settle product correctness. [SlopCodeBench paper](https://arxiv.org/html/2603.24755).

My inference: the factory needs three things together: clear architecture, executable checks, and a feedback loop that converts actual failures into better checks. A longer instruction file alone is insufficient.

## The recommended tools and their responsibilities

### 1. Oxfmt for formatting

A formatter chooses layout mechanically: spacing, wrapping, indentation, and related presentation. It removes an entire category of review arguments and makes generated code easier to compare.

Use Oxfmt as the sole formatter for its supported file types, pinned in Atlas. The current published version is 0.68.0; the latest formal maturity announcement found calls it beta. Qualify it on Atlas's TS/TSX, configuration, and documentation before the first formatting-only commit. Keep import sorting off initially because moving imports can change side-effect order. Do not mix formatting with semantic cleanup. [Oxfmt documentation](https://oxc.rs/docs/guide/usage/formatter), [sorting defaults](https://oxc.rs/docs/guide/usage/formatter/sorting), [beta announcement](https://oxc.rs/blog/2026-02-24-oxfmt-beta).

After qualification, format owned files immediately after edits and check the full first-party tree at integration. Exclude generated auth/Panda output, dependencies, retained worktrees, and build artifacts explicitly. CI checks formatting without modifying the candidate. Use shfmt for shell files, which are a separate language surface.

### 2. Oxlint plus a narrow ESLint lane

A linter finds specified code patterns: mistakes, unsafe constructs, and conventions that a compiler does not necessarily reject. Oxlint is the fast default for native JavaScript/TypeScript correctness checks. Current published version: 1.83.0. Enable a curated set of correctness and suspicious-code rules, not every stylistic and pedantic rule simultaneously. [Oxlint configuration](https://oxc.rs/docs/guide/usage/linter/config).

There is a material compatibility constraint: Oxlint's current stable type-aware engine requires TypeScript 7+, while Atlas pins 5.9.3. Trey has now required that migration in the re-baseline. After compiler/tool qualification, use Oxlint's native typed rules rather than first building a broad new typed-ESLint lane. Retain ESLint for Atlas's four custom rules and any demonstrated coverage gaps. Oxlint's JavaScript plugin API remains alpha and does not support plugins that need TypeScript type information; TS7 does not remove that limitation. Do not delete existing protections just because a migration tool ran successfully. [Type-aware requirements](https://oxc.rs/docs/guide/usage/linter/type-aware), [plugin limitations](https://oxc.rs/docs/guide/usage/linter/js-plugins), [migration limitations](https://oxc.rs/docs/guide/usage/linter/migrate-from-eslint).

Prioritize handled promises, correct async callbacks, exhaustive state handling, unsafe `any` flows, meaningful test assertions, and inappropriate suppression directives. Keep framework-specific overrides: Next requires certain default exports, and a test fixture can intentionally contain a rejected example. Map every rule to its owning engine so duplicate reports do not become duplicate blockers.

Add `noUncheckedIndexedAccess`, which makes a lookup acknowledge that the item might be missing, and `exactOptionalPropertyTypes`, which distinguishes omission from explicitly supplying `undefined`, after a diagnostic run establishes the work involved. Also evaluate implicit-return and switch-fallthrough checks. Preserve `tsc` as the type authority. These changes require a deliberate baseline migration, not blind autofixes or casts that hide the findings. [TypeScript indexed-access check](https://www.typescriptlang.org/tsconfig/noUncheckedIndexedAccess.html), [optional-property check](https://www.typescriptlang.org/tsconfig/exactOptionalPropertyTypes.html).

### Why Atlas is on 5.9 and what TS7 changes

The original choice was documented, not accidental. `docs/plans/rebuild-research/2026-09-07-pin-recon.md`, section 6, already recognized TypeScript 7.0.2 as stable but recommended 5.9.3 for M0. Its stated concern was TS7's missing JavaScript compiler API and undeclared compatibility in schema/auth/test tooling outside Next. Spec section 19 lists TypeScript 5.9; scaffold commit `9ddbdde` introduced the exact package pin on September 8. The research recommended a later TS7 compatibility lane, but current Beads searches for TypeScript and TS7 found no dedicated follow-up. That was a conservative bootstrap hold, not a permanent Atlas requirement.

TypeScript 7's largest change is a native compiler and language server, written in Go, with parallel work and improved watch behavior. Microsoft's benchmarks report roughly 8-12 times faster full builds on selected large repositories, not a promised Atlas speedup. Faster type feedback makes frequent, comprehensive checks more practical for agents. Editor find-references, diagnostics, and completion can benefit too, provided the editor actually selects the native language server. It does not make Atlas's running JavaScript application proportionally faster. [Microsoft release announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/), [current published package](https://registry.npmjs.org/typescript/latest).

For this factory, the direct tool unlock is Oxlint's `typescript-go`-based typed linting. It can catch forgotten promise handling, unsafe value flows, inappropriate async callbacks, and incomplete state handling through selected rules. These categories were already possible with typed ESLint; TS7 enables the fast native route we now want. Oxlint can also combine type errors and lint output in one invocation. Initially retain an independent native `tsc` check and demonstrate equivalent file/config/diagnostic coverage before considering consolidation. Oxfmt, ordinary Fallow analysis, and Specgate's architecture rules did not require TS7 and do not acquire new correctness guarantees from the upgrade. [Oxlint typed checks and combined mode](https://oxc.rs/docs/guide/usage/linter/type-aware).

The compatibility caveat still matters: TypeScript 7.0 has no general JavaScript compiler API. Microsoft provides a TS6 compatibility package and side-by-side installation pattern for tools that embed that API; the planned new 7.1 API is not a shipped 7.0 feature. A narrowly scoped compatibility dependency may be necessary even while TS7 is the authoritative compiler. Current Next docs support the project-local TS7 CLI, but do not establish compatibility for every generator, lint plugin, and type-test path in Atlas. Qualify Better Auth/Drizzle generation, Next build, tests, worker/CLI execution, and local tooling explicitly. [Microsoft compatibility guidance](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-60), [Next TS7 integration](https://nextjs.org/docs/app/api-reference/config/typescript#using-typescript-7).

The migration must make globals and configuration explicit, handle changed defaults and removed options, and avoid hiding new diagnostics behind casts or ignored build errors. Several stricter settings proposed above already exist in 5.9; enabling them is an intentional quality-policy change, not a feature newly invented in TS7. Pin native worker concurrency for the shared devbox rather than maximizing every agent's compiler threads independently. Record before/after results on the same tree so compiler improvements are distinguishable from added lint rules and later cleanup.

### 3. Fallow for repository health

Fallow is a separate project built with Oxc technology, not another official Oxc executable. It examines the repository graph for unused code and dependencies, repeated code, cycles, complex functions, and related maintainability risks. Use it as the primary whole-project analyzer. [Fallow project](https://github.com/fallow-rs/fallow), [analysis limitations](https://docs.fallow.tools/analysis/limitations.md).

Qualify the discovery first. Atlas has Next routes, DBOS registrations, command-line entrypoints, generated code, executable spikes, and several test projects. A missing entrypoint makes useful code look dead; declaring everything an entrypoint hides real dead code. Record the discovered file set, resolution errors, and exclusions. Test-only use is a review signal, not permission to delete a contract.

The published Fallow release is 3.26.0, but your installed deslop strictly pins 3.25.0. Choose one qualified version for the eventual setup. Either retain the installed version initially or qualify an explicit deslop/analyzer upgrade; do not run incompatible analyzer versions under one baseline. The new release includes stale-baseline and skipped-file improvements, but those flags cannot be assumed to exist in the installed adapter. [Fallow 3.26.0 release](https://github.com/fallow-rs/fallow/releases/tag/v3.26.0).

Use separate measurements rather than one health score:

- Cyclomatic complexity counts independent control-flow paths.
- Cognitive complexity estimates how difficult branching and nesting are to follow.
- Duplication identifies repeated token structure, not proof that two functions should share an abstraction.
- Hotspots combine code complexity with change history to direct attention, not to certify quality.

Proposed calibration settings: cyclomatic and cognitive complexity ceilings of 15 for ordinary production functions, with cyclomatic 10 as an earlier advisory; nesting depth 4 as a review trigger; exact-clone review at 50 tokens and 5 lines. These are starting proposals, not universal truths. A flat exhaustive dispatcher may earn an exception. Splitting a coherent operation into eight meaningless helpers does not earn quality credit.

Keep near-miss duplication and aggregate scores advisory. A clone finding must be resolved by a justified consolidation, a rewrite, or a documented intentional duplication, never an automatic extraction. Fallow's `health --min-score` can make individual complexity findings informational; do not use a high aggregate score as the blocking condition. [Health exit behavior](https://docs.fallow.tools/cli/health.md), [duplication semantics](https://docs.fallow.tools/analysis/duplication.md).

The initial cleanup requires whole-repository analysis. `fallow audit --gate all` still operates on changed files and cannot certify untouched files. After cleanup, changed-code checks provide speed; full scans preserve the baseline. [Audit scope](https://docs.fallow.tools/cli/audit.md).

### 4. Specgate for architectural rules

Your actual Specgate is the right kind of tool for preventing architectural drift. It declares module ownership, public entrypoints, allowed dependencies, and related boundaries, then checks the import graph. For Atlas, it can help keep domain logic independent of app entrypoints and stop adapters from reaching into unrelated internals. It also compares policy revisions to identify rules being loosened.

The local source is v0.3.2 at `5f47356`; an executable was not found on this shell's PATH or the usual checked build paths. More importantly, source inspection found that several strict-looking settings do not by themselves give strict enforcement:

- Parser and required-envelope problems can remain warnings.
- Ownership requires a separate doctor command.
- Inline-ignore expiry and new-ignore limits lack an identified runtime enforcement path.
- Policy-diff does not cover every possible configuration weakening.

These are source findings, not live negative-test results. Before adopting it as a hard gate, prove its actual behavior with harmless positive and negative fixtures. Require complete ownership, zero unexpected parse failures, explicit unresolved-local-import failure, and a full check without a blanket baseline. Disallow inline ignores until their behavior is qualified or corrected. Review all policy-file changes independently even when policy-diff says they pass.

Prefer Specgate as the architecture-policy owner. Keep the existing ESLint rules for expression-level checks. Use dependency-cruiser as a comparison or fallback if Specgate cannot express a required boundary; do not introduce three competing architectural configurations. Details and exact source references are in [the local-tool inspection](2026-09-16-quality-specgate-local.md).

### 5. Deslop for cleanup and independent review

Your deslop tooling is already installed. It combines pinned Fallow evidence, a persistent remediation record, native repository checks, and fresh review. Use its rehabilitation workflow for the pre-M2 cleanup. Each batch should remove a demonstrated problem while preserving observable behavior, with consumer tracing before deletion and fresh review before acceptance.

Use its prevention audit during M2, but configure it deliberately. A plain `deslop audit` can omit Atlas's automatically discovered aggregate gate because quick audit selects particular check names. At integration, require the full native-check scope and, for substantial or risky changes, `--require-review`. Do not put deslop inside the same gate that deslop calls; that would recurse. Its default changed-code audit is not a whole-repository clean certificate.

The existing receipt mechanism binds many source, policy, tool, and review inputs and rejects stale approvals. It does not freeze every installed dependency, external service, or environment variable. Extend the integration record with the lockfile, runtime/tool revisions, environment profile, source inventory, and exact candidate commit. The installed deslop entrypoint follows a live checkout, so recording `0.1.0` alone is insufficient. [Local implementation and limits](2026-09-16-quality-specgate-local.md).

Use a checked-in `.fallowrc.json` without remote configuration extensions. Explicitly qualify and enable the desired type-aware analysis and completeness policy there; deslop's rehabilitation requests semantic analysis, but its prevention invocation does not automatically carry that flag forward. If semantic evidence is required and unavailable, the result is incomplete, not permission to delete code.

Review should judge concept count and responsibility, not appearance alone: unnecessary wrappers, speculative abstractions, duplicated state ownership, generic frameworks for one concrete need, hidden fallback behavior, and tests that duplicate the implementation rather than the contract. Every accepted simplification should explain what a maintainer no longer has to understand. A single reviewer pass with no findings is not exhaustive proof.

### 6. Tests that detect wrong behavior

Add coverage with `@vitest/coverage-v8` matching the existing Vitest version. Coverage counts which implementation lines, branches, and functions the tests execute; it does not measure whether assertions are good. Explicitly include all owned implementation files, including files no test imports. Verify aggregation across Vitest projects so the final report is not simply the last suite's output. [Vitest coverage](https://vitest.dev/config/coverage.html).

Proposed targets for calibration: 90% lines/statements/functions and 85% branches, with module-specific floors so easy files cannot hide an untested critical file. Separately require every named authority, privacy, transaction, idempotency, and undo invariant to have a behavior test. That requirement is more important than a rounded percentage. Reject zero-test runs, focused tests, unexplained skips, broad coverage exclusions, and missing expected suites.

Use fast-check for property tests, which generate inputs and operation sequences and look for a counterexample to a general rule. Examples for M2 include:

- Saving a debrief changes every intended record or none.
- Repeating the same request does not create a second effect.
- Undo restores the eligible prior business state while version counters continue forward.
- A later edit causes a grouped undo to refuse without partially reverting other records.
- Resolving two conflicting claims does not erase an unseen third claim.
- Revoking access cannot make more data visible through a summary, count, artifact, or cached response.

Keep the reference model simpler and independently expressed; persist failing seeds and reduced cases. These future M2 properties do not require implementing M2 during cleanup. [fast-check model tests](https://fast-check.dev/docs/advanced/model-based-testing/).

Use StrykerJS for focused mutation testing: it deliberately makes a small implementation mistake and asks whether the tests notice. Begin with pure policy predicates, validation, and state transitions. Review surviving high-risk mutations, then promote a calibrated subset to a hard gate. Its Vitest runner's declared versions fit Atlas, but actual multi-project compatibility still needs a trial. [Stryker Vitest runner](https://stryker-mutator.io/docs/stryker-js/vitest-runner/).

Keep real-Postgres concurrency and DBOS crash tests. PGlite is useful for cheap tests but its single-connection behavior cannot establish production locking correctness; fast-check's promise scheduler cannot control a real database's locks. Require the full slow suites on the integrated candidate, including actual browser journeys where relevant. No Impeccable is proposed. Atlas's existing approval/login/settings surfaces still need keyboard, labeling, focus, and functional tests. [PGlite limitation](https://pglite.dev/docs/pglite-socket), [DBOS testing](https://docs.dbos.dev/typescript/tutorials/testing).

### 7. Security, dependencies, scripts, and migrations

Use a redacted local secret scanner in the staged-content check and a full history check for the baseline. Gitleaks is the established initial candidate; its current project says feature work has moved to Betterleaks. Qualify the successor on synthetic positive/negative fixtures before switching. Never enable online credential validation or print suspected secret values as part of ordinary quality work. [Gitleaks](https://github.com/gitleaks/gitleaks), [Betterleaks](https://github.com/betterleaks/betterleaks).

Use OSV-Scanner for dependency-advisory findings, with one policy for severity, applicability, and exceptions. Offline mode avoids sending dependency information out but needs a fresh local advisory database. A scan error or stale database is an incomplete result. Do not run several overlapping advisory scanners as separate sources of mandatory work, and do not use automatic dependency fixes as cleanup. [OSV offline mode](https://google.github.io/osv-scanner/usage/offline-mode/).

Atlas's pinned npm 11.19.0 already supports install-script allowlists, strict refusal of unapproved scripts, and a minimum release age. Use those instead of changing package managers. Qualify the needed scripts, including code generation, explicitly. A proposed seven-day age policy needs an exception route for urgent fixes and deliberate new-tool qualification; age is a delay, not evidence a package is safe. Preserve exact pins and `npm ci`, inspect signatures/provenance where available, and generate a release dependency inventory through npm's SBOM support. Scan the worker image separately. [Exact pinned npm configuration source](https://github.com/npm/cli/blob/v11.19.0/workspaces/config/lib/definitions/definitions.js), [install-script controls](https://github.com/npm/cli/blob/v11.19.0/docs/lib/content/commands/npm-install-scripts.md), [npm SBOM](https://docs.npmjs.com/cli/v11/commands/npm-sbom/).

Qualify one static security engine, preferably Opengrep with pinned local rules and Semgrep CE as a comparator. Start diagnostic, then promote demonstrated high-confidence findings. Rules can catch unsafe construction and accidental sensitive-data handling; they do not prove business authorization. Keep private source local, turn off optional telemetry, and record the engine and rule-pack licenses separately. [Opengrep](https://github.com/opengrep/opengrep), [Semgrep engine scope](https://docs.semgrep.dev/semgrep-code/semgrep-pro-engine-intro).

Add ShellCheck and shfmt for shell, actionlint for the checked-in GitHub-format workflow, and actual runner validation for Forgejo. Use Squawk selectively to review Postgres migration hazards. Online-upgrade restrictions should not force compatibility machinery into Atlas's pre-launch schema merely to satisfy a generic preset. Real migration application and database constraints remain the correctness controls. [ShellCheck](https://github.com/koalaman/shellcheck), [shfmt](https://github.com/mvdan/sh), [actionlint](https://github.com/rhysd/actionlint), [Squawk rules](https://squawkhq.com/docs/rules).

## How the enforcement should work

Use thin wrappers around existing tools, not a new quality framework.

| Boundary | Checks | Mutation policy |
| --- | --- | --- |
| After an edit | Format affected owned files; fast lint diagnostics | Formatting only after qualification; no automatic deletions or broad refactors |
| Before a task finishes | Typecheck, focused tests, changed-code health, policy-change warning | Check-only; failure returns actionable evidence to the agent |
| Before a commit | Staged formatting/lint and redacted secrets check | Inspect the actual staged content; never silently stage additional work |
| Before integration | Full native gate, whole-repo health/architecture checks, coverage, required crash/E2E, qualified security checks | Check-only on the exact integrated candidate |
| Before accepting risky work | Independent exact-change review and behavior evidence | Implementer cannot self-approve or alter the quality policy to get green |
| Periodic maintenance | Full mutation pass, deeper analysis, dependency advisories, cleanup review | Findings become bounded work; no unattended sweeping rewrites |

Codex and Claude Code currently expose post-tool hooks suitable for formatting. Their tool names and payloads differ. A Claude `Edit|Write` hook does not see shell writes; current Codex docs include `apply_patch` and shell/unified-exec paths. Test each installed harness and actual edit path rather than copying one JSON example into both. Hooks may run concurrently, so one ordered handler should format before linting a file, with per-worktree coordination. [Codex hooks](https://developers.openai.com/codex/hooks), [Claude hook guide](https://code.claude.com/docs/en/hooks-guide), [Claude hook reference](https://code.claude.com/docs/en/hooks).

Hook requirements: parse JSON without shell interpolation; validate canonical paths inside the current worktree; reject generated, secret, dependency, and out-of-scope paths; handle spaces safely; invoke pinned local binaries without implicit downloads; bound runtime; avoid recursive hook loops; and do not overwrite a newer concurrent edit. Use changed-file detection as a fallback for shell edits, without formatting another lane's work. A late formatter change invalidates earlier evidence. Keep typecheck and model review out of the per-keystroke path.

Compose with Beads' existing hooks rather than replacing `core.hooksPath` with Husky or Lefthook. A hook failure should be understandable and fixable, with a rule ID, location, expected property, and suggested correction. A timeout or unavailable analyzer is not a pass at the authoritative boundary.

For enforcement outside the implementing agent, use the estate's Forgejo integration path and qualify its runner/protection settings. The repository's manual GitHub workflow alone does not provide that. A contributor who can edit the verifier or bypass the only gate can weaken it; the policy and required-check definition therefore need separate review/control. This may require an explicit change to how main receives contributions. It does not authorize a GitHub push, service configuration, or branch-policy change now. [Forgejo protection](https://forgejo.org/docs/latest/user/repository/protection/), [workflow trust rules](https://forgejo.org/docs/latest/user/actions/security-pull-request/).

Specgate's policy-diff compares Git revisions, not uncommitted policy edits. The task/precommit check must separately flag changed policy files; integration compares the exact candidate commit with the approved base. A passing comparison against an older `HEAD` cannot approve later working-tree changes.

## Anti-slop rules that deserve hard enforcement

The first policy should prevent concrete failure modes:

1. Every owned source file is accounted for by the compiler, relevant analyzers, and an architecture module or explicit exception.
2. The required suite cannot pass after collecting zero tests or silently dropping an entire project.
3. Production cannot depend on tests or bypass the approved module interface; browser code cannot reach server-only implementations.
4. Unhandled async work, missing state cases, unvalidated external data, and unjustified type escapes fail their applicable checks.
5. Placeholder implementations on reachable product paths and tests that never assert the promised behavior are rejected by targeted tests and review.
6. A policy relaxation, new ignore, reduced coverage denominator, or regenerated baseline is a reviewed change, never an agent's routine fix.
7. All hard checks report analysis completeness and the actual tool/config revision; missing evidence fails closed.
8. No semantic autofix, dead-code deletion, dependency update, or abstraction extraction is accepted without behavior evidence.

Custom rules need valid and invalid fixtures, including legitimate cases that resemble the forbidden pattern. Existing ESLint is adequate for many Atlas-specific rules; ast-grep is an alternative if a structural rule is materially easier to express and test there. Avoid adding another rule engine for a problem already solved. [ast-grep rule-test design](https://ast-grep.github.io/guide/test-rule).

Do not hard-ban all `catch`, `any`, comments, wrappers, barrels, long files, or repeated tests. Each can be justified. The policy should reject specific defects and require reasons for risky exceptions, not train the agent to disguise code until a heuristic stops complaining.

## Other options worth considering

| Option | Distinct value | My recommendation |
| --- | --- | --- |
| Knip | Independent unused-code and framework-entry discovery | Compare with Fallow once during qualification; keep it only if it adds material coverage |
| dependency-cruiser | Rich dependency/reachability policy | Specgate qualification fallback or comparator, not a second policy owner by default |
| Biome | Integrated formatter/linter alternative | Viable alternative to Oxc, not an additional formatter |
| SonarJS | Additional bug and maintainability rules | Select non-overlapping rules only; avoid a second complexity gate |
| jscpd / scc | Duplication or lightweight multi-language metrics | Useful if Fallow lacks a needed language or measured capability; not required for this TS-heavy tree |
| StopSlop | Structural smells and code reachable only through its own tests | Diagnostic bakeoff candidate; its orphan heuristic needs framework/contract review |
| AntiSlop / AI-SLOP Detector | Stub, placeholder, and suspicious-pattern detection | Mine useful rules and qualify them; do not accept marketing claims as calibration |
| Desloppify | Broad cleanup campaign with model-based scoring | Compare if useful, but avoid a second remediation ledger beside your deslop |
| Qlty / MegaLinter | A unified interface to many existing analyzers | Not my first choice here; orchestration overlap and rule duplication exceed the demonstrated need |
| SonarQube / CodeScene | Dashboards, history, deeper quality analysis | Optional later comparison if they catch problems our local stack misses |
| Paid cross-file SAST / CodeQL | Deeper security-flow analysis | Evaluate separately for measured gaps, private-repo licensing, data handling, and operating cost |
| Renovate | Ongoing reviewed dependency maintenance on Forgejo | Useful follow-on with scoped credentials, not a prerequisite for baseline cleanup |
| TLA+ / Alloy or another state model | Exhaustively explore a bounded abstract transition model | Consider for a genuinely ambiguous undo/recovery design; not a general elegance checker |
| Husky / Lefthook / lint-staged | Convenient hook scheduling | Existing Beads hook ownership makes a small composed handler preferable initially |
| Nx / Turborepo / distributed caching | Faster repeated builds | Add only after measuring a bottleneck and proving cache inputs include policy, environment, and tools |

Primary comparisons and version details: [Oxc/Fallow research](2026-09-16-quality-oxc-fallow.md), [testing/security research](2026-09-16-quality-tests-security.md). Additional repositories read: [StopSlop](https://github.com/BaryshevRS/stopslop), [AntiSlop](https://github.com/skew202/antislop), [Desloppify](https://github.com/peteromallet/desloppify), [Qlty](https://github.com/qltysh/qlty). These broader options are not all Atlas-tested recommendations. In particular, Qlty's current BSL license has explicit restrictions involving third-party code-quality and AI-coding services; do not assume its advertised free CLI is an unrestricted factory dependency. This is a procurement flag, not a legal determination about Atlas. [Qlty license](https://github.com/qltysh/qlty/blob/main/LICENSE.md).

## How to reach the clean baseline

The work should proceed in this order once its scope is approved:

1. **Lock the policy and preserve behavior.** Decide exact tool versions, source scope, thresholds, exceptions, and who can change them. Record existing functional defects separately. Cleanup does not silently authorize the M2 approval-boundary change or other proposed product semantics.
2. **Qualify the compiler and tools before judging Atlas.** Include the required TS7 migration, native typed lint, and any narrowly needed legacy-API tool dependency. Use harmless fixtures to demonstrate both rejection and acceptance. Include a missing analyzer, empty test suite, undiscovered entrypoint, forbidden import, type escape, ignored file, intentional clone, weak assertion, and stale review receipt. Inspect output, not only exit codes.
3. **Capture the full baseline.** Run the existing gate and every new whole-repository analysis on a fixed revision. Record incomplete checks as incomplete. Do not create a blanket baseline that accepts every current finding.
4. **Make the formatting-only change.** Keep its diff separate, run appropriate checks, and prevent later semantic work from hiding inside it.
5. **Repair coherent groups of findings.** First address behavior/authority/type defects within the approved scope, then verified dead code, duplicated responsibilities, dependency boundaries, complex state logic, and weaker tests. Trace consumers before deletion. Preserve readable transactions and public contracts.
6. **Review the actual simplified architecture.** A fresh reviewer checks the whole product flow and remaining conceptual surface, not just analyzer output. Unknown purpose blocks deletion; justified exceptions stay visible.
7. **Establish the accepted baseline.** Full checks on the integrated revision, complete source/test discovery, no unresolved mandatory findings, no unreviewed policy changes, and an explicit exception/uncertainty record. Known functional defects cannot be hidden behind the phrase slop-free.
8. **Update M2 against the new tree.** Cleanup may change interfaces, file ownership, line references, tool versions, and verification commands. Reconcile and relint the plan, reapply Beads, and re-review consequential changes before launching it.

The same checks then become build-time requirements. One coordinator runs the full gate on the integrated candidate; workers run focused checks in isolated worktrees. This avoids both stale green evidence and multiple agents simultaneously running expensive full suites.

## Make the experiment measurable

Use a small fixed corpus of Atlas-shaped tasks and intentionally faulty fixtures, plus a held-out set the configuration work does not tune against. Include both real problems and clean counterexamples. Keep compiler, model route, starting revision, dependencies, and resource limits fixed when comparing configurations.

Compare the current stack with the proposed layers on isolated copies, not by weakening the live project's required gate. Remove one optional diagnostic at a time to see what it contributes. Look at repeated feature changes as well as one-shot fixes; the question is whether the code remains easier to change through M2.

Record:

- Real defects caught, missed, and introduced by remediation.
- False-positive rate after adjudication, including valid code that the tool encouraged us to make worse.
- Agent repair attempts, repeated failures, and requests to relax a rule.
- Gate runtime, compute/model cost, and delay before useful feedback.
- Escaped regressions, flaky checks, and incomplete-analysis incidents.
- New concepts, public APIs, dependencies, cycles, and concentrated complexity, not just lines removed.
- Suppression counts and reasons, reviewer disagreements, and changes rejected for gaming a metric.

Each added blocker should earn its place by catching an important failure reliably. Each detector should have a named owner, known limitations, and a tested failure contract. Keep the results in the existing repository receipts and work ledger rather than building a second observability product.

## Decision to make before implementation

My proposed package is the Oxc/Fallow/Specgate/deslop setup above, strengthened behavioral tests, local security checks, and a qualified integration gate outside the implementing agent's discretion. Use the alternatives as a bounded comparison set, not a mandate to install everything.

The unresolved choices are the exact strictness thresholds, whether to qualify Fallow 3.26 with deslop now or begin with its installed 3.25, which Specgate gaps to repair versus guard externally, and the authoritative Forgejo enforcement arrangement. Those belong in a dedicated pre-M2 quality plan. TS7 migration is now required scope; exact tool pins, implementation settings, remote protections, and cleanup execution remain unapproved. M2 G1 remains open.

Research method: fresh Exa search/content retrieval, Firecrawl developer lookup, npm registry and first-party repository/release inspection, Atlas configuration/source inspection, and separate Oxc/Fallow, local-tool, and testing/security research passes. Published availability, installed availability, source-level findings, and runtime proof are kept separate throughout. The three supporting notes contain further source URLs and qualification detail.
