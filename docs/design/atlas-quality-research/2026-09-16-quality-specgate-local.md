<!-- Provenance: copied 2026-09-16 from the Atlas repository (docs/plans/rebuild-research/2026-09-16-quality-specgate-local.md), where Astra wrote it as pre-M2 quality-tooling research for Atlas. It inspects Specgate v0.3.2 and the deslop and Fallow tools as installed locally. Copied into Warrant unchanged except that local filesystem paths were generalized to ~/ form. The Atlas-specific rulings inside apply to Atlas, not to Warrant; Warrant's spec cites this file for the tool survey and the reasoning. -->

# Atlas quality tooling: local Specgate and deslop

Research date: 2026-09-16. Scope: source inspection and installed `--help`/`--version` only. No code analyzer, repository gate, model review, installation, cleanup, or remediation ran. No changes were made to either tool repository. Recommendations below are proposals, not approved quality policy.

## Recommendation

Use Trey's Specgate to enforce Atlas's architectural boundaries, Fallow to measure dead code, duplication and complexity, and deslop to manage behavior-preserving cleanup with fresh review. These are complementary jobs. None establishes that a codebase is objectively free of slop.

The local tools are more capable than a generic tool list would suggest. Deslop already has exact-change review receipts, analyzer pinning, incomplete-evidence protections, and a remediation ledger. Specgate has useful policy-change detection that catches architectural rules being relaxed. But several Specgate settings are weaker than their names suggest, and a plain deslop audit can omit Atlas's native gate. Qualification must test the actual commands and failure cases before treating them as mandatory gates.

For a clean starting point, require no unresolved mandatory findings across the entire repository, with narrowly recorded exceptions. Afterward, use fast changed-code checks during editing and full-repository checks at integration. A falling number of findings or a health score of 100 is not a substitute for those requirements.

## Verified local inventory

| Component | Observed state | Evidence |
|---|---|---|
| Specgate source | Clean checkout at `5f47356ffa6490b1c297bc4839d212c270684f3d`, branch `master`, package version `0.3.2` | `~/Code/specgate/Cargo.toml:1-6`; `git status`, `git log -1` |
| Specgate provenance | `origin` is `https://github.com/treygoff24/specgate.git` | `git remote -v`; this is still a GitHub-gated remote |
| Specgate executable | Not on this shell's PATH; no binary at the usual `.cargo/bin/specgate` or checkout `target/{release,debug}/specgate` paths checked | `command -v specgate` and exact-path existence checks; not a machine-wide absence claim |
| Deslop source | Clean checkout at `7aa2d1effd965cc3d62350756f5ad7af9dd69012`, branch `main` | `~/Code/deslop-tooling`; `git status`, `git log -1` |
| Deslop provenance | Forgejo `origin`, project `estate/deslop`; GitHub mirror `treygoff24/deslop` | `git remote -v` |
| Deslop executable | Installed, reports `deslop 0.1.0`; live symlink to checkout entrypoint | `~/.local/bin/deslop` -> `~/Code/deslop-tooling/bin/deslop` |
| Fallow executable | Bundled with deslop, reports `fallow 3.25.0`; no ambient PATH executable found | `~/Code/deslop-tooling/node_modules/.bin/fallow --version` |
| Fallow pin | Exact `3.25.0`; wrong runtime version is refused | Deslop `package.json:14`, `src/deslop/oracles.py:22,245-270,329-354` |

These are installed/local revisions, not a claim about the newest public release. In particular, a newer public Fallow version does not automatically become the version deslop supports. Upgrading the analyzer requires checking schema changes, evidence comparison and baseline migration, then requalifying the gate.

## What Specgate enforces

Specgate is a Rust program using Oxc's JavaScript/TypeScript parser and resolver. A module spec identifies a part of the codebase, its public entry files and the modules it may depend on. The checker turns imports into a dependency graph and compares that graph with the declared rules.

Its implemented rules include import allow/deny lists, public-entry restrictions, canonical imports, layer/category direction, circular dependencies, export-name uniqueness, test/production import hygiene, and named boundary contracts. Ownership diagnostics find files without a module, overlapping module claims, invalid patterns and duplicate module IDs. The monorepo resolver uses the nearest TypeScript configuration and discovers npm/pnpm workspace packages. [Local source: `~/Code/specgate/src/rules/mod.rs`, `src/spec/ownership.rs`, `src/spec/workspace_discovery.rs`; operator entrypoint: `SPECGATE_FOR_AGENTS.md`.]

For Atlas, this can preserve the intended division between domain logic, database implementation, external-service adapters and entrypoints such as MCP, web and CLI. It can prevent a new adapter from reaching through a public interface into another module's internals. The policy should reflect actual ownership and approved dependency direction; adding a new layer merely to satisfy a checker would defeat the purpose.

`policy-diff` compares module specs and project configuration between Git revisions, classifying changes that widen, narrow or restructure the rules. Deleting a policy spec is treated as widening; ambiguous rename/copy comparisons are conservative. Exit codes are 0 for no widening, 1 for widening and 2 for an evaluation error. This deserves a protected integration gate because agents should not solve a failing architecture check by quietly relaxing its rules. [Local source: `src/policy/config_diff.rs`, `src/policy/classify.rs`, `src/cli/policy_diff.rs`; reference: `docs/reference/policy-diff.md`.]

Specgate does not judge whether an abstraction earns its existence, whether a function implements the business rule correctly, whether a test is meaningful, or whether duplicated-looking code should be consolidated. Its envelope check looks for particular validation imports and calls; it does not prove that validation controls every execution path.

## Specgate qualification: settings versus effective enforcement

The following are source-level findings. They were not reproduced against a running Specgate binary in this research task.

| Area | Effective behavior in this checkout | Recommended Atlas treatment |
|---|---|---|
| Main check | New error-level violations fail. Warnings alone pass. | Assign mandatory architecture rules error severity; inspect structured output as well as exit status. |
| Baselines | Existing baseline entries suppress matching findings in ordinary checks. `--no-baseline` disables this classification. | Prefer `--no-baseline` for the clean-slate/full gate. Do not automatically regenerate a baseline when a build fails. |
| Stale baseline entries | Default is warning; `stale_baseline: fail` can block them. | If retaining a temporary baseline, require metadata and expiration under a separate review, and enable stale-entry failure. |
| Ownership | `strict_ownership` is consumed by `doctor ownership`, not implicitly added to `check`. With level `warnings`, all ownership findings gate. | Invoke the ownership doctor explicitly; an uncovered source file must not silently escape architectural policy. |
| Governance conflicts | `doctor governance-consistency` is implemented and exits 1 on conflicts. Older operator-guide passages still describe it as backlog. | Use the command after qualification, not the stale roadmap description. |
| Unresolved imports | Default `unresolved_edge_policy` is `warn`; `error` is available. Bare package specifiers are classified as external. | Use error policy for unresolved local edges. Keep package resolution, typecheck and dependency checks because this is not a complete missing-package detector. |
| Parser failures | TS/JS parser errors become nonfatal `parse_warnings`; the overview doctor reports their count. | Add a completeness check requiring zero unexpected parse warnings, alongside TypeScript and lint coverage of every relevant source file. |
| Required envelope validation | Missing validation and analysis failures are emitted as warnings. | Do not rely on this as the sole enforcement of Atlas's authorization or validation requirements. Use explicit behavior tests and narrowly selected AST rules where useful. |
| Inline ignore expiration | Expiry is parsed, but source references show no runtime expiry consumer. Graph construction suppresses the import when any ignore comment exists. | Do not rely on `require_expiry` to make exceptions expire. Disallow inline Specgate ignores in Atlas until this is qualified or corrected. |
| New-ignore budget | `escape_hatches.max_new_per_diff` and `require_expiry` are config fields with policy-diff handling, but no runtime enforcement consumer was found. | Treat these settings as unqualified, not as a functioning safety control. |
| Baseline metadata | `baseline.require_metadata` is enforced by `baseline add` and `baseline audit`; it is not a main-check precondition. | Explicitly audit a retained baseline. Prefer no baseline after rehabilitation. |
| Policy-diff coverage | The local implementation does not cover every weakening: `baseline.require_metadata`, test-boundary settings and envelope matcher details are absent from `classify_config_changes`; test-pattern changes are structural. The reference claims broader metadata coverage. | Require review of all quality-policy/configuration changes in addition to Specgate's classifications. Never describe it as a complete anti-bypass boundary. |

Source pointers for this table:

- `src/verdict/mod.rs:353-365` and `src/cli/check.rs:314-330`: main verdict and optional baseline loading.
- `src/cli/doctor/ownership.rs:114-123`: ownership gate.
- `src/cli/doctor/governance_consistency.rs:47-101`: actual conflict gate.
- `src/cli/analysis.rs:35-105,164-187`: unresolved edge classification.
- `src/parser/mod.rs:122-150`, `src/cli/doctor/overview.rs:59`: parse warnings.
- `src/rules/contracts.rs`, tests named `required_envelope_*_reports_warning`: envelope severity.
- `src/parser/ignore.rs:9-48`, `src/graph/mod.rs:628-640`, `src/rules/boundary.rs:552-557`: ignore handling.
- `src/spec/config.rs:250-257`, repository-wide identifier searches for the two escape-hatch fields: field definitions and their only non-test uses in policy comparison.
- `src/cli/baseline_cmd.rs:457-476`, `src/policy/config_diff.rs:15-153`: metadata audit and policy comparison coverage.

Before adoption, qualification should demonstrate that the chosen commands reject an invalid import direction, internal-file import, uncovered source file, module cycle, conflicting policy and unresolved local import. It should also confirm what happens when analysis is incomplete, an exception is added, a baseline is regenerated or a policy/config file changes. These are tool-contract tests on harmless fixtures, not a request to weaken Atlas to test it.

Use one policy-widening gate, not both. A full-source check with `--no-baseline` plus a separate `policy-diff --base <approved-base>` is clearer than using an incremental `check --since` as the only repository-wide gate. Retain the explicit ownership and consistency commands. Use the actual branch base rather than a stale local `origin/main`.

## Deslop rehabilitation and prevention are different operations

Deslop's rehabilitation mode is a controlled cleanup campaign. `run` discovers native checks, captures analyzer evidence, invokes a fresh reviewer and creates persistent work items under `.deslop/`. `fix` selects a coherent batch but does not edit product code. The implementing agent makes the changes; `verify` reruns evidence and obtains a separate review before resolving the work. This is a good fit for the proposed pre-M2 cleanup, provided the quality policy and observable behaviors to preserve are locked first.

The fresh review examines unnecessary interfaces, pass-through wrappers, duplicate concepts, speculative compatibility, misplaced module boundaries and tests that preserve implementation details. Those are questions a complexity counter cannot settle. Review coverage is self-reported and findings are capped at 12 per pass, so a single empty result is not a repository-wide clean certificate. [Local sources: `~/Code/deslop-tooling/prompts/initial-review.md:7-13`, `schemas/review.schema.json:39`, `src/deslop/scope.py:38-59`.]

Prevention mode is `deslop audit --base <ref>`. It runs Fallow's changed-code audit and whitespace checks, plus selected native checks. It deliberately tolerates inherited debt; therefore it cannot establish the requested clean starting point. `--full` includes every configured native check. `--require-review` adds independent review of the exact proposed change. Without that flag, there is no model review and no architectural approval. [Local sources: `src/deslop/cli.py:2003-2095`, `README.md:166-209`.]

There is an important integration trap. Discovery prefers a combined `verify`, `gate` or `check` script and records it with the name `verify`. Ordinary audit selects only checks named `lint`, `typecheck` or `security`. A repository with just the discovered aggregate gate can consequently have no native check selected by plain audit. Atlas should use `--full` at the integration boundary or explicitly configure the quick check set. Do not put a deslop audit inside the same native gate that deslop calls; that would recurse. [Local sources: `src/deslop/repository.py:355-402`, `src/deslop/cli.py:2041-2045`.]

### What already protects evidence

- Fallow is an exact tool-owned pin, not an ambient PATH fallback. The installed copy is 3.25.0.
- Analyzer failures and malformed output do not count as clean findings. Incomplete dead-code evidence, type-aware degradation and unread files are handled explicitly in remediation verification.
- Review receipts bind source, policy, analyzer configuration, verifier source, Python identity, Fallow binary/version and retained artifacts.
- Whole-change review requires a matching token, unchanged inputs and a single-use acceptance record.
- A new reviewed audit supersedes the old pending proof. Tools or source changing between checks and approval invalidate acceptance.
- Suppression, deferral and a verified fix are distinct dispositions. Findings that reappear can reopen; a suppression is not a fix.

Evidence: `src/deslop/oracles.py:329-354`; `src/deslop/fallow_evidence.py`; `src/deslop/evidence.py:131-148`; `src/deslop/audit_evidence.py:34-135`; tests in `test_audit_evidence.py`, `test_fallow_evidence_guards.py` and `test_fallow_upgrade.py`. These tests were inspected, not run here.

### Limits to preserve in Atlas's gate design

Deslop is not a sandbox. Native checks execute trusted repository scripts. The Codex/delegate runners request read-only behavior from their underlying harness; a custom command runner needs its own restrictions. Receipts do not freeze external services, every environment variable or transitive installed dependencies. Because the installed entrypoint follows a live checkout, a deslop source update can change behavior without changing its `0.1.0` version string. Record the tool source revision as well as `--version` and avoid upgrades during an active review.

Rehabilitation automatically requests type-aware dead-code analysis for TypeScript and can retain a syntactic fallback as incomplete evidence. Prevention invokes `fallow audit --base` without an explicit `--type-aware` flag. The target's pinned Fallow config must request semantic analysis and the desired completeness policy; do not assume the rehabilitation setting carries over. [Local sources: `src/deslop/oracles.py:390-430,661-669`.]

Use checked-in `.fallowrc.json` for this experiment. It is explicitly included in deslop's proof inputs. The current input enumerator lists `.fallowrc.json`, `.fallowrc.jsonc` and `fallow.toml`, while installed Fallow also accepts `.fallow.toml` and config extension mechanisms. A tracked file is still part of the tree hash, but an ignored configuration or external extension needs separate treatment; avoid those extra inputs rather than claiming they are all sealed. [Local sources: `src/deslop/evidence.py:131-148`; installed Fallow 3.25.0 `audit --help`.]

Fallow's installed help explicitly distinguishes the gating `audit` command from `review`/`--brief`, which always exit 0. It also says runtime hot-path coverage is license-gated and informational. Neither belongs in a mandatory quality gate under the assumption that its exit status rejects defects.

## Proposed use in the Atlas experiment

1. Agree on the mandatory findings, preserved behavior, source inventory, generated-file exclusions and exception process. Start with architectural boundaries that Atlas already needs, not a new module hierarchy invented for the tools.
2. Qualify the selected toolchain on small contract fixtures and Atlas's actual monorepo resolution. Pin source revisions, package versions and configuration. Confirm both successful analysis and rejection of deliberately bad fixture cases.
3. Run a full baseline campaign, not just changed-code audit. Triage findings into actual defects, high-confidence cleanup, justified exceptions and unknowns needing better evidence. Keep tests and behaviors fixed while making coherent, independently reviewed changes.
4. Reach a stated clean condition: full native gate passes; full required static checks pass; all mandatory findings are resolved; exceptions are explicit and narrow; fresh review has recorded coverage and remaining uncertainty. Do not promise mathematical proof of elegance or zero undiscovered defects.
5. Keep fast feedback while editing, deterministic full gates at integration, and exact-change fresh review for substantial or risky changes. Never turn a slower model review into a format-on-edit hook, and never let a high health score overrule a failed behavior test.

The existing same-week survey at `~/Code/deslop-tooling/docs/research/2026-09-14-slop-detector-universe.md` is a useful source map for alternatives. Its claims about third-party tools were not revalidated in this local-inventory subtask. The proposed polyglot slop-engine vision beside it is research, not a shipped feature to include in Atlas's gates.
