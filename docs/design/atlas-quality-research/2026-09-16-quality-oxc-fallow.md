<!-- Provenance: copied 2026-09-16 from the Atlas repository (docs/plans/rebuild-research/2026-09-16-quality-oxc-fallow.md), where Astra wrote it as pre-M2 quality-tooling research for Atlas. It surveys Oxc, Fallow, and overlapping analyzers. Copied into Warrant unchanged except that local filesystem paths were generalized to ~/ form. The Atlas-specific rulings inside apply to Atlas, not to Warrant; Warrant's spec cites this file for the tool survey and the reasoning. -->

# Atlas quality tools: Oxc, Fallow, and overlapping analyzers

Research checked 2026-09-16 UTC. Recommendations, not an approved implementation plan. No Atlas dependencies were installed, no code scan was run, and no application files were changed for this research.

Later scope ruling, 2026-09-16 23:42 UTC: Trey requires TS7+ migration in the re-baseline sprint, without authorizing execution. The original recommendation below to retain TS5.9 initially is superseded by the [updated main report](2026-09-16-atlas-quality-factory.md); the compatibility findings remain relevant.

## Recommendation

Use Oxfmt for formatting, Oxlint for fast local correctness rules, a narrow ESLint lane for Atlas's existing custom rules and TypeScript-aware checks, and Fallow for whole-project dead code, duplication, and complexity. Give each signal one blocking owner. Add dependency-cruiser only if the architecture policy needs reachability or dependency-type constraints that the existing rules and Fallow cannot express adequately. Use Knip as an independent discovery check during initial qualification, not a second permanent dead-code gate by default.

The important mismatch is TypeScript. Atlas pins 5.9.3; current Oxlint type-aware linting is stable but uses TypeScript 7 semantics and explicitly requires TypeScript 7+. Adopting Oxlint's ordinary lint rules does not require turning on its type-aware engine. Retain the current compiler and add typed ESLint checks first, unless we deliberately approve and qualify a TypeScript 7 migration. [Oxlint type-aware requirements](https://oxc.rs/docs/guide/usage/linter/type-aware), [typescript-eslint typed setup](https://typescript-eslint.io/getting-started/typed-linting).

"No unresolved findings under a tested policy" is an achievable clean starting point. "Objectively slop-free" is not a conclusion these tools can establish. They can measure branches, repeated tokens, imports, and specific unsafe patterns. They cannot establish that an abstraction is necessary, that a feature serves Atlas's mission, or that a passing test asserts the right behavior.

## What is published now

Exact versions below were read from the npm registry on September 16, not inferred from search snippets. Published does not mean installed or qualified in Atlas.

| Tool | Published version | Status and license | Atlas implication |
| --- | --- | --- | --- |
| Oxfmt | 0.68.0, September 14 | Published; latest formal maturity announcement found is beta; MIT | Suitable candidate for one pinned formatter, with fixture qualification and format-only initial diff |
| Oxlint | 1.83.0, September 14 | Published stable 1.x; MIT | Candidate for native lint rules |
| oxlint-tsgolint | 7.0.2001, July 21 | Type-aware engine announced stable July 22; MIT | TypeScript 7 compatibility decision required |
| Fallow | 3.26.0, npm September 15; GitHub release September 16 | Published stable release tag; MIT static tooling | Candidate whole-project analyzer; very active and relatively young project |
| Knip | 6.36.0, September 16 | Published 6.x; ISC | Independent dead-code/dependency comparator |
| Biome | 2.5.14, September 16 | Published 2.x; MIT OR Apache-2.0 | Alternative formatter/linter stack, not another layer beside Oxfmt/Oxlint |
| dependency-cruiser | 18.3.1, September 14 | Published 18.x; MIT | Richer architecture graph policy if needed |
| eslint-plugin-sonarjs | 4.2.1, September 15 | Published; LGPL-3.0-only | Selective rules only, with license recorded; not a default full preset |

Version and license sources: [Oxfmt registry](https://registry.npmjs.org/oxfmt), [Oxlint registry](https://registry.npmjs.org/oxlint), [tsgolint registry](https://registry.npmjs.org/oxlint-tsgolint), [Fallow registry](https://registry.npmjs.org/fallow), [Knip registry](https://registry.npmjs.org/knip), [Biome registry](https://registry.npmjs.org/@biomejs/biome), [dependency-cruiser registry](https://registry.npmjs.org/dependency-cruiser), [SonarJS registry](https://registry.npmjs.org/eslint-plugin-sonarjs). Release corroboration: [Oxc September 14 release](https://github.com/oxc-project/oxc/releases/tag/apps_v1.83.0), [Fallow 3.26.0](https://github.com/fallow-rs/fallow/releases/tag/v3.26.0), [Oxfmt beta announcement](https://oxc.rs/blog/2026-02-24-oxfmt-beta), [typed lint stable announcement](https://oxc.rs/blog/2026-07-22-type-aware-linting-stable).

The independent local-tooling research found Deslop currently pins and bundles Fallow 3.25.0 and rejects other versions. That is different from published Fallow 3.26.0. Do not upgrade a binary underneath Deslop or assume new upstream flags are available through its current adapter. That installation observation is reported by the sibling research lane; this lane verified upstream releases.

## What Oxc adds

Oxc is a compiler-tooling family. Oxfmt formats source; Oxlint checks source. Fallow is a separate project that uses Oxc's parser and semantic machinery, not another official Oxc executable.

### Oxfmt: one formatter, automatic feedback, no style arguments

Oxfmt supports TS/TSX, JavaScript, JSON variants, YAML, TOML, Markdown/MDX, HTML, CSS variants, GraphQL, and other formats. Its maintainers report full conformance on their Prettier JavaScript/TypeScript tests. That is a compatibility claim, not proof that every Atlas file will have an identical or behavior-preserving rewrite. Its release still contains fixes to comment placement and formatting edge cases. [Supported workflow](https://oxc.rs/docs/guide/usage/formatter), [current release fixes](https://github.com/oxc-project/oxc/releases/tag/apps_v1.83.0).

Recommended configuration choices:

- Pin the project dependency and use that exact version in editor, agent hook, and CI.
- Format only the edited first-party file after an agent edit. Run repository-wide `--check` in the canonical gate. CI checks; it does not silently rewrite the submission.
- Exclude generated Panda output, generated auth schema, build artifacts, coverage, and retained worktrees. Keep first-party tests and tooling in formatting scope.
- Start with formatting only. Qualify side-effect import ordering before enabling import sorting.
- Do not run a second formatter over the same languages. Keep a separate formatter only for a genuinely unsupported file type.

Import sorting is disabled upstream by default. Package-field sorting is enabled by default, so decide explicitly whether its first diff is wanted. Tailwind sorting is not relevant merely because the formatter offers it; Atlas uses Panda.

Oxfmt's ignore rules differ in consequential ways: `ignorePatterns` cannot be overridden by explicitly naming a file, whereas Git-ignored files can still be formatted when explicitly named. Its global Git ignore file is not read. Lockfiles are always ignored. Hook path validation and explicit generated-file exclusions therefore matter even when `.gitignore` looks sufficient. [Sorting defaults](https://oxc.rs/docs/guide/usage/formatter/sorting), [ignore semantics](https://oxc.rs/docs/guide/usage/formatter/ignore-files).

### Oxlint: fast correctness checks, selected strictness

Oxlint's current documentation advertises more than 865 rules across core JavaScript, TypeScript, React, import, test, accessibility, and other rule families. Correctness rules are enabled by default; suspicious, performance, and pedantic rules are separate choices. Enabling everything is not a coherent quality policy: some rules embody competing style preferences, and more warnings can make real failures harder to see. [Oxlint overview](https://oxc.rs/docs/guide/usage/linter), [configuration and categories](https://oxc.rs/docs/guide/usage/linter/config).

For Atlas, favor rules that catch unhandled asynchronous work, impossible or redundant logic, accidental coercion, unsafe type escape hatches, incorrect test assertions, and forbidden dependencies. Use a curated strict ruleset rather than `all`, with production, test, config, and framework-entry overrides. Next.js requires some default exports; a blanket no-default-export rule would contradict the framework rather than improve the architecture.

Oxlint has native cyclomatic complexity, nesting-depth, function-line-count, and parameter-count rules. Cyclomatic complexity counts independent paths through a function; nesting depth counts blocks inside blocks. Neither measures whether extracting another helper would make the program easier to understand. If Fallow owns complexity thresholds, avoid a competing Oxlint complexity gate; use Oxlint for immediate nesting and correctness feedback. [Complexity](https://oxc.rs/docs/guide/usage/linter/rules/eslint/complexity), [nesting](https://oxc.rs/docs/guide/usage/linter/rules/eslint/max-depth), [function length](https://oxc.rs/docs/guide/usage/linter/rules/eslint/max-lines-per-function), [parameters](https://oxc.rs/docs/guide/usage/linter/rules/eslint/max-params).

Oxlint also has a project module graph for import rules, including cycle detection, and Oxc-specific rules for duplicated branch bodies and oversized barrel files. A barrel re-exports other modules through one file. These are useful diagnostics, but we should preserve intentional package boundaries rather than ban all re-exports. [Multi-file analysis](https://oxc.rs/docs/guide/usage/linter/multi-file-analysis), [barrel rule](https://oxc.rs/docs/guide/usage/linter/rules/oxc/no-barrel-file.html), [shared branch code](https://oxc.rs/docs/guide/usage/linter/rules/oxc/branches-sharing-code.html).

### Typed checks and custom plugins: the migration boundary

Type-aware linting asks the TypeScript checker what an expression means, rather than inspecting syntax alone. This catches a promise that is created but never handled, `any` flowing into trusted values, and an async function supplied where a synchronous callback is expected. Oxlint's stable typed engine implements 59 of typescript-eslint's 61 typed rules. It is tied to TypeScript 7.0.2; that numbering is deliberate, not an arbitrary plugin major version. Atlas currently has TypeScript 5.9.3 and `typescript-eslint` 8.70.0. Its present ESLint config uses `recommended`, not `recommendedTypeChecked`, and has no project-service typed-lint setup. [Stable engine release](https://oxc.rs/blog/2026-07-22-type-aware-linting-stable), [compatibility requirements](https://oxc.rs/docs/guide/usage/linter/type-aware).

Recommendation: keep `tsc` authoritative, retain ESLint, and add the selected typed rules there using the project's compiler. Oxlint can take over native untyped rules after parity checks. Later, a separately qualified TS7 migration could allow Oxlint's typed engine to replace that lane. Root-only type-aware options, declaration availability between workspaces, and generated Next/Panda files must be correct before its diagnostics are trusted. Do not claim a successful scan of only the files it happened to assign to projects covers every source file.

Oxlint's JavaScript plugin API remains alpha. It supports much of ESLint v9's API, and maintainers test plugins including SonarJS, Playwright, Testing Library, and React Hooks. It still does not support plugin rules that need TypeScript type information or custom parser/file-format integrations. Atlas's four local ESLint AST rules must remain in force until their existing positive and negative fixtures pass through any replacement. The migration tool does not automatically migrate local custom plugins. [Plugin support and omissions](https://oxc.rs/docs/guide/usage/linter/js-plugins), [migration guidance](https://oxc.rs/docs/guide/usage/linter/migrate-from-eslint).

Do not remove a duplicated ESLint rule merely because Oxlint supports its name. Confirm that the Oxlint rule is enabled with equivalent options and file scope. The upstream side-by-side migration pattern is useful; it is not an excuse for accidental coverage gaps.

## What Fallow adds

Fallow analyzes connections across the repository: unused files and exports, dependency declarations, import cycles, architecture boundaries, repeated code, and complex functions. Its ordinary static analyses are deterministic; the same inputs and configuration produce stable findings. Its health score combines several measurements into a ranking, which remains a heuristic judgment even when its arithmetic is deterministic. [Project and command inventory](https://github.com/fallow-rs/fallow), [analysis limitations](https://docs.fallow.tools/analysis/limitations.md).

This is the strongest fit for the specific AI-code risks Trey describes: new helpers nobody calls, near-copies with renamed variables, abstractions accumulated across unrelated tasks, and huge functions hidden inside a superficially tidy module. It cannot determine the business value of a helper or prove that a dynamically invoked workflow is dead.

### Qualify what the graph sees before deleting anything

Atlas has four npm workspaces, Next app routes, a worker entry, a CLI entry, DBOS registration, generated schema/styling, Vitest projects, migration/config files, and executable spikes. Entry points are files the runtime or a tool can start from without an ordinary source import. Missing one makes reachable code appear unused; declaring every file an entry point makes dead code disappear from the report.

Fallow understands npm workspaces and many Next conventions, but its config analysis cannot evaluate arbitrary JavaScript. Explicitly inventory the unusual Atlas entry points and inspect discovered files, plugins, workspace resolution, and exclusions. Tests need their own scope; they are not disposable because production-mode analysis omits them. An export used only by tests is a review question, not an automatic deletion order. [Workspace support](https://docs.fallow.tools/configuration/workspaces.md), [Next-specific analysis](https://docs.fallow.tools/frameworks/react.md), [limitations](https://docs.fallow.tools/analysis/limitations.md).

Two current discovery traps deserve a fixture: most hidden directories are not traversed, and 3.26.0 excludes directories named `build` at any depth. First-party code under those paths may be absent rather than clean. The release adds `--explain-skipped` evidence and workspace diagnostics, which should be inspected rather than suppressed indiscriminately. [3.26.0 release and upgrade notes](https://github.com/fallow-rs/fallow/releases/tag/v3.26.0).

Optional Fallow type-aware analysis can confirm exact symbols through aliases and re-exports and report API coupling. Results distinguish complete, partial, and unavailable evidence. It is not a substitute for the compiler or typed lint. Require complete evidence only after the optional version-matched companion has been qualified for Atlas's projects. A partial result must never turn into permission to delete a symbol. [Type-aware boundaries](https://docs.fallow.tools/analysis/type-aware.md).

### Complexity and duplication: useful measures, easy to game

Fallow reports both cyclomatic complexity and cognitive complexity, a measure that penalizes difficult control flow and nesting. Upstream default ceilings are 20 and 15 respectively. I would try a production policy of cyclomatic 15, cognitive 15, and nesting depth 4, with an advisory prompt at cyclomatic 10. These are proposed engineering limits, not scientifically proven boundaries between elegant and bad code. First inspect the actual distribution. A flat exhaustive dispatcher can deserve a named exception; splitting a transaction into eight one-use helpers purely to satisfy a number can make Atlas worse. [Health metrics and thresholds](https://docs.fallow.tools/cli/health.md).

Duplication has four modes. Mild and strict currently produce the same exact-token results; weak normalizes literal values; semantic also normalizes identifiers. "Semantic" here does not mean the tool proves two functions have the same behavior. Near-miss analysis is also deterministic, but the choice to merge its findings is a design judgment. The separate model-backed `similar-code` workflow labels its results unverified and should remain advisory. [Duplication semantics](https://docs.fallow.tools/analysis/duplication.md).

Recommended starting policy: exact clones of at least 50 tokens and 5 lines are blocking review findings in production code, not automatic demands to extract an abstraction. Report semantic and near-miss clones for review. Allow intentional clone groups only by stable fingerprint, instance count, and recorded reason so a new copy reopens the finding. Test setup duplication needs separate treatment; preserving readable independent tests can be preferable to sharing a helper that duplicates the implementation's mistake.

Do not target a health score of 100 or a duplication percentage of zero. The score is an aggregate; a small dangerous function can disappear inside a good average. `fallow health --min-score N` also changes exit behavior: individual complexity findings become informational unless another explicit severity gate catches them. Use explicit per-rule blockers and keep the score for trends. Coverage-weighted risk should use actual Istanbul coverage, not Fallow's estimated test-reachability proxy. [Health exit contracts](https://docs.fallow.tools/cli/health.md).

### Clean starting point versus changed-code gate

`fallow audit` is a changed-file gate. Its default compares new findings with a base revision; `--gate all` includes inherited findings in changed files. Neither is a full-repository clean-slate certificate. The initial cleanup needs whole-project dead-code, duplication, and health runs under the final scope. Later, changed-code analysis speeds feedback while a full-repository check still protects the accepted baseline. Pin the base SHA; a guessed merge base or last-commit diff can conceal issues introduced earlier on a branch. [Audit scope and attribution](https://docs.fallow.tools/cli/audit.md).

Fallow 3.26.0 adds `--fail-on-stale-baseline`, but it deliberately declines to judge some narrowed runs and `audit`. Do not treat this flag alone as proof every exception remains valid. Prefer no blanket baseline at the end of the cleanup: every surviving finding is either fixed, an evidenced detector mismatch, or a specific accepted exception. Keep suppression inventories visible. JSDoc visibility tags such as `@public` and even `@internal` can suppress unused-export reports; newly adding such tags to make CI green needs review. [Release contract](https://github.com/fallow-rs/fallow/releases/tag/v3.26.0), [suppression semantics](https://docs.fallow.tools/configuration/suppression.md).

## Which alternatives earn a place

| Tool | Distinct value | Recommendation for Atlas |
| --- | --- | --- |
| Knip | Dedicated unused-file/export/dependency analysis, detailed framework/plugin and script discovery | Run once alongside Fallow during qualification; investigate disagreements. Choose one ongoing blocking owner per dead-code finding. Keep Knip if its discovery handles Atlas-specific conventions materially better. |
| dependency-cruiser | Declarative forbidden, allowed, and required graph rules; reachability, runtime/type-only distinctions, dependency categories, and graph reports | Worth adopting if those extra policy dimensions are needed. Fallow zones alone do not prove every file has a policy: unmatched files and zones without rules are unrestricted. Test classification completeness whichever tool wins. |
| SonarJS | Additional bug patterns, cognitive complexity, and code smells | Select only proven non-overlapping rules. Fallow can own complexity; typed SonarJS rules need ESLint because Oxlint JS plugins do not supply type information. Record LGPL dependency rather than assuming MIT. |
| Biome | Integrated formatter, linter, import organizer, and custom-rule ecosystem | A credible alternative to the Oxc stack, not a reason to install two formatters. Its current floating-promise rule remains in nursery; do not assume parity with TypeScript-checker-based rules. |
| jscpd | Focused duplication detector | Fallow's own current benchmark says jscpd v5 is faster for pure duplication. No need to add it unless Fallow misses needed language coverage or performance becomes a measured problem. |

Primary comparisons: [Knip Next plugin](https://knip.dev/reference/plugins/next), [Knip workspaces](https://knip.dev/features/monorepos-and-workspaces), [Knip production mode](https://knip.dev/features/production-mode), [dependency-cruiser rules](https://github.com/sverweij/dependency-cruiser/blob/main/doc/rules-reference.md), [dependency-cruiser TypeScript caveats](https://github.com/sverweij/dependency-cruiser/blob/HEAD/doc/faq.md), [Fallow zone behavior](https://docs.fallow.tools/analysis/boundaries.md), [SonarJS rule/API scope](https://github.com/SonarSource/SonarJS/blob/master/packages/jsts/src/rules/README.md), [Biome current promise rule](https://biomejs.dev/linter/rules/no-floating-promises/), [Fallow duplication comparison](https://docs.fallow.tools/analysis/duplication.md).

Vendor timing claims are useful for choosing what to try, not evidence of Atlas performance. Benchmark the qualified configs on the same checkout, with cold/warm runs, discovered file counts, and no hidden reduction in scope.

## Auto-fixes and enforceable outcomes

Formatting is appropriate for an edit hook. Structural deletion is not. Oxlint separates safe fixes, suggestions, and dangerous fixes; only reviewed safe-fix behavior belongs in an automatic hook. Keep suggestions and dangerous fixes explicit. Fallow fixes can remove exports, dependencies, and declarations, so use dry-run, inspect the consumer evidence, apply a bounded change, and run focused plus full gates. Its auto-fixable label is not proof of runtime safety. [Oxlint fix categories](https://oxc.rs/docs/guide/usage/linter/automatic-fixes), [Fallow fix behavior](https://docs.fallow.tools/analysis/auto-fix.md).

The quality policy itself needs tests. A useful qualification suite introduces one known violation per enforced property in isolated fixtures, checks the expected rule/path and nonzero gate result, then checks the valid counterpart. Include a missing entry point, a forbidden cross-package import, an unhandled promise, an intentional generated file, a hidden first-party hook, and a clone that gains a third copy. If the report is empty because nothing was scanned, the gate must fail.

Acceptance should mean: exact tool versions, verified file scope, zero unadjudicated configured failures, no unreviewed rule relaxations or suppression growth, unchanged behavior backed by the existing and strengthened tests, and measured feedback cost. That is a defensible foundation for M2. A prettier score badge alone is not.
