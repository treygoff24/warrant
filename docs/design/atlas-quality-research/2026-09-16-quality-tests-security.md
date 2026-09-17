<!-- Provenance: copied 2026-09-16 from the Atlas repository (docs/plans/rebuild-research/2026-09-16-quality-tests-security.md), where Astra wrote it as pre-M2 quality-tooling research for Atlas. It covers tests, architecture rules, and security controls. Copied into Warrant unchanged except that local filesystem paths were generalized to ~/ form. The Atlas-specific rulings inside apply to Atlas, not to Warrant; Warrant's spec cites this file for the tool survey and the reasoning. -->

# Atlas quality controls: tests, architecture, and security

Research checked 2026-09-16. Recommendations, not approved configuration. This work read local source and current primary documentation; it did not install packages, run tests or scanners, change application code, or change Beads.

## The strongest improvements build on tools Atlas already has

Atlas already has useful defenses. `scripts/gate.sh` runs strict TypeScript, ESLint, the size budget, tooling tests, two schema variants of core tests, web tests, real-Postgres workflow tests, the MCP conformance probe, and a Next.js build. `eslint-rules/` contains four repository-specific rules, with fixtures and tests, covering DB pool construction, DBOS imports in core, schema-qualified names, and nondeterministic imports in workflows. `packages/core/src/contracts/contracts.test-d.ts` already checks exact type contracts.

There is also a substantial crash suite. `apps/worker/tests/crash/proven.ts` runs deliberately defective and production variants, and fails when an assertion cannot distinguish them. `tests/acceptance.d/03-crash.sh` requires named crash cases and observed red/green results. This is an existing, unusually valuable defense against plausible but ineffective tests. Stryker should supplement it, not replace it.

The gaps visible in configuration are narrower than "we need a testing stack":

- `vitest.config.ts` has no coverage configuration. Most projects set `passWithNoTests: true`; only the crash project explicitly rejects an empty suite.
- `npm run gate` omits crash and browser E2E. The separate acceptance runner requires crash testing; Atlas acceptance does not omit it.
- `.github/workflows/ci.yml` is manual-only and invokes `npm run gate`. No Forgejo workflow was found locally; remote branch protections and separate runners were not checked.
- The current custom lint rules inspect specific imports and constructions. They do not establish a whole-program dependency graph or prove runtime privacy and transaction behavior.

The recommended outcome is a measured clean baseline with no unadjudicated findings under declared checks. "Objectively slop-free" is not a property these tools can certify. A formatter can settle layout; a test can establish a particular behavior; neither can certify that every abstraction earns its place.

## Recommended enforcement order

| Control | Recommendation | What it establishes, and what it misses |
| --- | --- | --- |
| Existing typecheck, custom rules, tests, build | Keep as hard gates; strengthen discovery | Preserves current contracts. A suite collecting zero tests must fail once that suite is required. |
| Vitest V8 coverage | Add a hard gate after measuring the baseline | Counts executed lines, branches, and functions. Explicitly include all owned implementation files; otherwise never-imported files disappear from the denominator. |
| fast-check properties and state models | Add targeted hard tests | Searches combinations and operation sequences that example tests miss. The model must be simpler and independently expressed, not copied implementation logic. |
| Real Postgres and DBOS crash/concurrency tests | Keep and extend as required integration/acceptance gates | Exercises locks, transaction conflicts, durable recovery, and persisted effects. PGlite and promise scheduling are not substitutes. |
| StrykerJS mutation testing | Diagnostic during baseline cleanup, then a focused hard gate | Makes small changes to implementation and checks whether tests detect them. Score alone does not establish complete requirements coverage. |
| Dependency-cruiser | Hard gate for a small set of declared graph rules | Enforces allowed dependency directions and forbidden runtime cycles. Dynamic resolution and framework entry points require calibration. |
| Gitleaks CLI | Hard gate with redacted output | Detects known secret patterns. Carefully review fixture exceptions; no broad baseline that silently accepts genuine credentials. |
| Betterleaks | Qualify alongside Gitleaks, then choose one authoritative scanner | Promising current successor; disable outbound secret-validation behavior during normal development checks. |
| OSV-Scanner | Hard adjudication gate on applicable dependency findings; scheduled fresh scan | Checks known advisories, not malicious code in otherwise unflagged dependencies. Keep build-time dependencies in scope too. |
| Native npm supply-chain controls | Hard gate | Lockfile fidelity, narrowly approved install scripts, and checked signatures reduce avoidable installation risk. A valid signature does not prove benign code. |
| Opengrep or Semgrep CE | Diagnostic first; tested, high-confidence local rules become hard gates | Static application security testing finds suspicious code patterns and data flows. It does not prove authorization correctness. Choose one engine for routine gates. |
| ShellCheck, shfmt, actionlint | Hard gates on applicable files | Protect the scripts that run all other checks. Actionlint checks GitHub Actions semantics, not full Forgejo runtime behavior. |
| Squawk | Advisory now; selected rules hard when relevant | Reviews Postgres migration hazards. Online-migration restrictions should not force unnecessary compatibility machinery into this pre-launch project. |

## Coverage that cannot hide untested files

Use `@vitest/coverage-v8` pinned to Atlas's Vitest version, currently 5.0.0. Vitest supports line, statement, function, branch, per-file, and path-specific thresholds. Its default source set contains files imported by the test run, so an explicit implementation `include` list is essential. Exclude generated/vendor code by exact, reviewed patterns, not broad directories added to make the number green. [Vitest coverage configuration](https://vitest.dev/config/coverage.html), [published 5.0.0 provider metadata](https://registry.npmjs.org/@vitest/coverage-v8/5.0.0).

My proposed calibration targets are 90% lines/statements/functions and 85% branches across owned implementation, with per-file floors and 100% of the enumerated policy decisions covered in authority, undo, idempotency, and source-access tests. These are proposed engineering thresholds, not published evidence that those percentages make software safe. Raise module thresholds where feasible. Do not rewrite readable code or insert empty assertions to reach a percentage.

Coverage needs one defined denominator across the monorepo. Different Vitest projects writing over the same report directory can leave a misleading last-suite result. The implementation should prove report aggregation and fail if a required project or named invariant disappears. Use explicit CI run mode, reject focused tests, and require reasons for skips and coverage-ignore comments. Do not let automatic threshold updates change policy during an ordinary gate run.

## Property tests for Atlas's actual risks

**Property testing** generates many inputs and checks a general rule. **Model-based testing** generates sequences of operations and compares the real system with a small reference model. fast-check supports both, shrinking a failure into a smaller reproducible example. It also supports scheduled asynchronous operations; its scheduler controls wrapped promise resolution, not Postgres lock scheduling or arbitrary network activity. [Model-based testing](https://fast-check.dev/docs/advanced/model-based-testing/), [race-condition scheduler](https://fast-check.dev/docs/advanced/race-conditions/).

Recommended Atlas properties, expressed at contract level:

- A repeated idempotent request has the same persisted effect; a reused key with different content is rejected.
- Debrief application changes all intended members or none. An eligible grouped undo restores the relevant prior business state without rolling version counters backward or erasing later work.
- Undo cannot remove a pre-existing memory merely because a consolidation receipt refers to its surviving ID.
- Adding an unrelated conflicting claim cannot silently erase it during resolution of the original pair.
- Revoking access cannot increase what the actor can retrieve through search, summaries, counts, artifacts, or cached-result replay.
- Equivalent transport calls preserve core semantics; CLI, ordinary MCP tools, and MCP Apps do not acquire different authority through different encodings.
- Changes in time zone, daylight-saving transitions, duplicate delivery, and out-of-order calendar updates preserve occurrence identity and deduplication rules.

Persist failing seeds and reduced cases as regressions. Run a bounded, reproducible set on every relevant change and a larger rotating sample periodically. Do not claim the finite sample proves every possible sequence.

Keep PGlite for cheap ordinary database tests. Its own docs state that it is single-connection; the socket multiplexer remains different from normal Postgres. Atlas already has a Docker Postgres harness, so reuse it for true multi-connection undo/write races, transaction conflict handling, and real process recovery. DBOS's testing guide likewise distinguishes mock-based unit tests from integration tests requiring Postgres. [PGlite connection limitation](https://pglite.dev/docs/pglite-socket), [DBOS testing](https://docs.dbos.dev/typescript/tutorials/testing).

## Mutation testing measures whether tests notice wrong behavior

StrykerJS is available now, including a Vitest runner. Registry metadata checked today reports core and runner 10.0.0, Node >=22, and runner peer support for Vitest >=2. Atlas's Node 24 and Vitest 5 fit the declared ranges; an actual sandbox run still has to establish compatibility with Atlas's multi-project setup. [Runner documentation](https://stryker-mutator.io/docs/stryker-js/vitest-runner/), [current runner metadata](https://registry.npmjs.org/@stryker-mutator/vitest-runner/latest).

Start with pure validation, authorization predicates, state-transition functions, version checks, and idempotency helpers. Do not begin by repeatedly booting the entire deployed journey for every mutant. Require a reviewed explanation for every surviving high-risk mutant; a numerical floor can supplement that review. Equivalent mutants, where the change cannot alter observable behavior, are a real source of noise.

Two important configuration traps:

1. The Vitest runner's `related` mode uses imports to select tests. API tests that do not directly import implementation need a different selection strategy or `related: false`.
2. Incremental reports do not automatically notice every environmental change, dependency update, snapshot, or non-mutated source change. Invalidate the cache on relevant inputs and require a full fresh run at the baseline and final acceptance. [Runner selection](https://stryker-mutator.io/docs/stryker-js/vitest-runner/), [incremental limitations](https://stryker-mutator.io/docs/stryker-js/incremental/).

## Architecture checks should encode Atlas, not generic aesthetics

Dependency-cruiser supports forbidden/allowed/required dependencies, unresolved imports, circular dependencies, and reachability. Suggested initial policies are core must not depend on app packages, browser bundles must not reach server credentials/provider implementations, workflows must not bypass the designated effect adapters, and runtime cycles require explicit approval. Existing local ESLint rules still cover expression-level constraints a dependency graph cannot see. [Rules reference](https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md).

Reuse current exact-type and registry tests rather than adding a second type-testing library. Extend their inventory checks when M2 adds resources, tools, presenters, and compensation handlers. A new tool or undoable entity with no transport, source-access, or compensation test should be a failed gate, not merely missing documentation.

Avoid hard limits on import fan-out or an arbitrary ban on all shared modules. Those measures can encourage agents to scatter related logic across more files while improving the score.

## Secret scanning and dependency security

Gitleaks remains a local MIT-licensed scanner, but its current README says new feature development has ended and future releases will be security patches. Its original author is now developing Betterleaks. Betterleaks 1.8.1 is a published non-prerelease, with contextual filtering and optional HTTP-based credential validation. [Gitleaks repository](https://github.com/gitleaks/gitleaks), [Betterleaks repository](https://github.com/betterleaks/betterleaks), [Betterleaks release](https://github.com/betterleaks/betterleaks/releases/tag/v1.8.1).

Use the CLI directly, not the separately licensed Gitleaks GitHub Action wrapper. Start with redacted staged-content scanning, then full tracked-history scanning at baseline and release. Inspect ignored-path coverage deliberately; do not indiscriminately publish scanner reports containing credentials. A real credential finding calls for containment and rotation decisions, not just deleting the detected line. Betterleaks should first be compared on synthetic positive and negative fixtures, without sending candidate secrets to external services. [Gitleaks Action licensing](https://github.com/gitleaks/gitleaks-action/blob/master/LICENSE.txt).

OSV-Scanner supports lockfiles, SBOMs, and container images. Its offline mode matches against a downloaded local database without transmitting project/dependency information. The database still needs freshness controls; a year-old offline database is not a current clean bill. Online dependency lookup and `npm audit` involve disclosure of dependency metadata. [OSV source scans](https://google.github.io/osv-scanner/usage/scan-source), [OSV offline mode](https://google.github.io/osv-scanner/usage/offline-mode/), [npm audit](https://docs.npmjs.com/cli/v11/commands/npm-audit/).

Prefer one vulnerability-finding policy with preserved advisory IDs, affected dependency paths, dispositions, and expiring exceptions. Do not make the same advisory generate three independent blockers through npm audit, OSV, and another scanner. Fresh actionable findings need review; a stale database or scanner error is an incomplete check, never a green security result. Do not run automatic dependency-fix commands as cleanup.

Atlas's pinned npm 11.19.0 already contains `allowScripts`, `strict-allow-scripts`, and `min-release-age`. Use pinned per-package install-script approvals and make unreviewed scripts fail installation. A proposed seven-day release-age window reduces exposure to very new releases, with an explicit exception path for urgent security fixes. Do not set a blanket "allow all scripts" escape hatch. These features were verified against the exact pinned source, not inferred from newer npm documentation. [npm 11.19.0 config definitions](https://github.com/npm/cli/blob/v11.19.0/workspaces/config/lib/definitions/definitions.js), [npm 11.19.0 install-script approval documentation](https://github.com/npm/cli/blob/v11.19.0/docs/lib/content/commands/npm-install-scripts.md).

Add `npm audit signatures` during dependency qualification and generate an SPDX or CycloneDX **software bill of materials**, an inventory of exact dependencies, for release artifacts. The first checks registry signatures and available provenance; the second supports later incident response. Neither establishes that the package's behavior is safe. Avoid adding a separate SBOM generator when native npm covers the JS dependency inventory; the worker container needs its own image/package coverage. [npm signatures](https://docs.npmjs.com/cli/v11/commands/npm-audit/), [npm SBOM](https://docs.npmjs.com/cli/v11/commands/npm-sbom/).

A self-hosted Renovate setup supports Forgejo and can propose grouped, age-delayed updates. It is a useful maintenance follow-on, not a prerequisite to cleaning this tree, and requires separately configured credentials and write authority. [Forgejo support](https://docs.renovatebot.com/modules/platform/forgejo/), [minimum release age](https://docs.renovatebot.com/key-concepts/minimum-release-age/).

## Static security analysis without false assurance

Semgrep CE is LGPL-2.1 and analyzes within a function. Semgrep's proprietary engine adds cross-function and cross-file analysis. Semgrep-maintained rules use a separate rules license allowing internal business use; third-party rules retain their own licenses. With local rules, metrics are not automatically enabled; explicitly turn metrics off anyway for reproducible local policy. [Engine scope](https://docs.semgrep.dev/semgrep-code/semgrep-pro-engine-intro), [licensing](https://docs.semgrep.dev/licensing), [metrics](https://docs.semgrep.dev/metrics).

Opengrep is an LGPL-2.1 fork with Semgrep-compatible rules, JSON/SARIF output, and improved within-file taint analysis, including flows between methods. Release 1.30.0 shipped September 7. Its within-file mode must not be described as complete whole-program cross-file analysis. [Opengrep README](https://github.com/opengrep/opengrep/blob/main/README.md), [1.30.0 release](https://github.com/opengrep/opengrep/releases/tag/v1.30.0).

I would qualify Opengrep with a pinned local rule set and retain Semgrep CE as the comparison option. Evaluate vendor-maintained rules against Atlas fixtures before promotion. Rules for dangerous SQL construction, uncontrolled redirect destinations, unsafe process execution, and accidental secret logging can catch useful mistakes, but business authorization and source provenance still require Atlas-specific tests. Each custom rule needs known-positive and known-negative fixtures. A scanner that reports nothing because parsing failed or files were excluded must fail qualification.

Paid cross-file analysis remains an option if a measured comparison finds meaningful misses in the local stack. No evidence gathered here establishes that its marginal value justifies another service for Atlas. Tooling-license facts above are metadata, not legal advice about redistribution or a commercial software-factory offering.

## Protect the scripts and migrations too

ShellCheck catches shell syntax and semantic mistakes; shfmt makes shell formatting deterministic. Both are useful for the shell-heavy Atlas gate and acceptance scripts. Actionlint validates GitHub workflow structure, expressions, action inputs, and embedded shell through ShellCheck. If Forgejo is the authoritative build location, test the resulting workflow on that runner too; a GitHub-oriented linter is not a Forgejo conformance test. [ShellCheck](https://github.com/koalaman/shellcheck), [shfmt](https://github.com/mvdan/sh), [actionlint](https://github.com/rhysd/actionlint).

Squawk is a Postgres migration linter that focuses on operations that block reads/writes or break clients. It supports explicit PostgreSQL versions, transaction assumptions, and narrow rule suppressions. Use it to review migration risks, while making the initial-empty-database versus live-upgrade distinction explicit. Keep actual migration application on real Postgres as the hard correctness test. SQL formatting alone does not prove migration safety. [Squawk rules](https://squawkhq.com/docs/rules), [CLI configuration](https://squawkhq.com/docs/cli).

Relevant local-tool license metadata: Vitest, fast-check, dependency-cruiser, Gitleaks, Betterleaks, and actionlint are MIT; StrykerJS, OSV-Scanner, and Squawk are Apache-2.0; shfmt is BSD-3-Clause; ShellCheck is GPL-3.0. These can run locally without a hosted code-upload service. Source links above and npm registry metadata were checked; operational availability is separate. Only ShellCheck, Gitleaks, and actionlint shims were observed on this host's PATH, and no scanner runtime or compatibility run was performed.

## What should not become mandatory yet

Do not add a second test framework, a second PostgreSQL container harness, a replacement type-contract framework, or several equivalent dependency scanners. Atlas already owns the essential testing foundations.

Keep whole-codebase mutation testing, broad uncalibrated security rules, paid code-analysis dashboards, and production-migration restrictions in the diagnostic or later category until their findings justify their costs. Consider a small formal state-machine model only if the review of grouped undo or recovery reveals ambiguity that tests cannot resolve. No new formal-methods platform is required merely to make the tooling list longer.

The baseline is ready when the agreed controls have run on the exact intended revision, every finding is fixed or narrowly adjudicated, required tests cannot silently disappear, and selected deliberate faults demonstrate that the new checks actually reject what they claim to reject. The same checks must then run outside the editing agent's discretionary hook path before integration.
