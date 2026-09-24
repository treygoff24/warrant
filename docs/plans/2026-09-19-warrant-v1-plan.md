# Warrant v1 implementation plan: M0 to M2 and the R14 checkpoint

Date: 2026-09-19. Author: Fable, coordinating agent, from the ratified v1 spec (`docs/specs/2026-09-16-warrant-v1-spec.md`, version 0.3, R17 at `7b30330`). Reader: Trey, the reviewing agent (Astra), and the execution lanes. Base: `main` at `02e7054`.

Status: draft for Astra's review and Trey's plan approval. Nothing here authorizes a build; Trey's word after plan approval does (AGENTS.md standing order; spec R17). Every technical claim in the spec that the spec itself marks unverified (S1, S3 to S7, before-decision delivery, the R14 comparison) stays unverified here and is carried as a task with an exit criterion, not as a fact.

## Why this plan stops at R14

The spec's build sequence (section 21) puts a stop-and-rethink checkpoint between M2 and M3: R14 says that if the early advisory comparison shows no benefit, the guidance is revised rather than the protected acceptance system built. Planning M3 to M5 in executable detail before that result would compile work the checkpoint may cancel, and D5's protected deployment cannot be planned before it is chosen. So this plan compiles M0, M1, and M2 fully, ends at the R14 judgment gate, and lists in "Carried forward" every human gate and evidence condition the next plan must open with. Trey can override this fence at plan approval.

## Goal lock: Warrant M0 to M2

## 1. Objective, in my words

Build Warrant from an empty repository to the point where it gives an agent a map before an edit, checks the edit's actual source against human-approved contracts with a four-fact advisory verdict, lets a human sign rulings that agents cannot forge, and then measures on Atlas whether the map changes how agents work. The build ends by putting that measurement in front of Trey.

## 2. Who and what it is for

Coding agents, who read the map through the CLI and one verified harness hook. Trey, who signs rulings and decides at R14 whether the guidance earns the protected gate. The execution lanes, who need every task to have a checkable close condition. Atlas is the first customer and the corpus's first real repository.

## 3. Acceptance demo

Runnable at the plan's close, on the integrated candidate at HEAD, from a clean checkout with the pinned toolchain. Each command is wired into `tests/acceptance.sh`, which is the plan's real acceptance and never a stub.

1. `scripts/gate.sh` passes: format, clippy with warnings denied, `cargo deny`, `cargo test --locked`, schema regeneration with no diff, the conformance suite, per-crate budgets, and `cargo-mutants` over `crates/core` and `crates/authority` with zero surviving mutants.
2. `warrant snapshot --index` in a repository whose worktree differs from its index prints the index tree id, and the conformance case proves the unrelated worktree bytes were never read; `warrant snapshot --worktree` on a file rewritten mid-capture exits with `snapshot-unstable` and writes no artifact.
3. `warrant inventory` on the fixture with one first-party file outside every module selector lists it under `unowned_source`; a colocated `*.test.ts` inside a module keeps class `test`.
4. `warrant query owner`, `warrant query consumers`, and `warrant context "add grouped undo to the meeting debrief"` on the pinned Atlas corpus return source references labeled by basis, and the S7 report states found, missed, misleading, and ambiguous counts with denominators against Trey's frozen sample. `retrieval.qualified` is true only if the predeclared criteria were met.
5. The delivery record for the one supported harness adapter shows, from a recorded harness run, context in model input before the decision, plus the recorded timeout and skipped-hook cases marked unavailable.
6. `warrant check` on the story A, B, and C fixtures produces three verdicts whose facets differ in the expected way: a reuse suggestion with no compliance failure, a policy change with `approval: required`, and an unread in-scope file with `analysis: incomplete`; exit code 1 in the last two and 0 in the first.
7. `warrant rule verify` agrees with `ssh-keygen -Y verify` on every S4 fixture across ed25519, ecdsa, and rsa keys, and rejects the same tampered inputs; `warrant rule sign` renders the record's effects from its final canonical bytes, asks for confirmation, writes nothing when the human declines, refuses to run without a human-supplied key path, and never reads a key from the repository.
8. `warrant policy diff` classifies a removed contract as widening, an added invariant as narrowing, a file move as restructuring, and a glob replaced by today's exact file list as uncertain.
9. `warrant verify` on an unsigned check receipt reports every step and the result `advisory`; the same receipt signed with the fixture producer key reports `verified-with-unchecked` without `--recompute` and `verified` with it, and is still not an acceptance credential because its predicate is `check`; the receipt reformatted with two spaces of indentation fails at step 1.
10. The early comparison report (`docs/acceptance/2026-M2-early-comparison.md`) records raw counts and denominators for the six outcomes in spec section 17.5, both arms, with retries and interventions visible, and the R14 checkpoint bead carries Trey's decision.

Green on these proves the checker follows its rules on the named inputs. It does not prove deployed runner isolation (M3), held-out product benefit (M4), or that any Atlas rule set is the right one.

## 4. Rulings I will make unless you override

| # | Ruling | Why | Cost if wrong |
|---|---|---|---|
| P1 | The plan covers M0 to M2 and ends at the R14 judgment gate; M3 to M5 compile into a second plan after that decision | R14 may cancel or reshape M3; D5's deployment is unchosen | A second planning round before M3; no build work is lost |
| P2 | Executors are subscription-backed apex models from one family: Astra at `low` first, Sol at `xhigh` as the retry slot. Reviewers are drawn only from other families (Opus 5, Cursor Grok 4.6, GLM 5.3); the adjudicator is Opus 5 `xhigh` with GLM behind it | Trey's experiment: whether high-quality first attempts shrink the review and fix loop. Astra `low` built the `show` crate from a settled spec in one pass (delegate fleet notes, 2026-09-14; an estate observation, not an independent benchmark). Keeping both executor routes in the OpenAI family means no retry can put the executor's family in a review or adjudication seat | If Astra `low` stalls, Sol `xhigh` retries and the task's ceiling parks it; if both fail, the task parks for the coordinator. The experiment record shows the cost either way, including stalled and abandoned tasks |
| P3 | Standard regime is three review seats from three families (refuter, conventions, test-skeptic), matching Trey's 2026-09-18 Atlas ruling; critical regime applies to `crates/authority`, the receipt verifier, the widening classifier, facet derivation, and every milestone gate, per spec 18.2 | Same review shape as the last comparable build, so the executor experiment has a comparable baseline; the spec names the critical surfaces | More review cost per task; the experiment's fix-round counts are measured against the same regime shape |
| P4 | Private corpus artifacts (Atlas tarball, dependency bundle) live in a local directory named by `WARRANT_CORPUS_DIR`, referenced by digest from `tests/corpus/manifest.yaml`; no new repository until a second machine needs them | D4 requires private storage; a multi-hundred-megabyte tarball does not belong in git; no home paths may enter the public repo | Corpus is single-machine until a fetch script is written; the manifest already carries the digests a second copy must match |
| P5 | CI is `.github/workflows/ci.yml` running `scripts/gate.sh` on stable and on the pinned MSRV; the script is the authority and CI mirrors it. CI must be observed executing before W0.G closes; that needs one GitHub push, which stays gated on Trey's word | Spec 17.1 requires every fixture in CI on every change and 18.2 requires MSRV and stable builds; local runs are not that claim | If no runner executes the workflow, W0.G reports a blocked prerequisite instead of closing; the local gate still runs at every integration and at close |
| P6 | Ownership devices: the core crate pre-declares its module tree and one schema stub per emitted document at W0.2, the CLI registers every section 12.1 command at W0.7 as an exit-2 `not-implemented` stub in its own file, and W0.2 adds the dependencies this plan's slice needs after rechecking versions, licensing, and runtime identity, leaving optional and deferred entries (SCIP, judgment providers, importers) out until their adoption task | Parallel lanes must not both edit `lib.rs`, `main.rs`, or `Cargo.toml`; spec 19 calls its versions candidate pins and makes optional entries non-mandatory | A lane that needs a dependency the recheck did not add reports it in `discovered` and the coordinator re-sequences; a pin that fails the recheck costs integration rework in W0.2, not later |
| P7 | Crate packages are named `warrant-<crate>` under `crates/<crate>`; the binary is `warrant` from `warrant-cli` | Unambiguous `cargo test -p` targets in every verify row | A rename is mechanical before any published or signed identity depends on the names; after M3 it is not |
| P8 | S4 (ssh-key versus `ssh-keygen` oracle) is the authority task's fixture suite, not a separate spike; S5 (RLIMIT on macOS) is carried forward to M3 because the devbox cannot measure macOS and no M2 task credits limits as applied | The spike's exit criterion is the fixture suite itself; S5 concerns the evidence executor, which M3 deploys | The attest wrapper records `limits: not-applied` until S5 runs. Deferring S5 does not defer ordinary child-process deadlines, cancellation, or truthful execution reporting, which W2.4 delivers |
| P9 | Evidence adapters in M2 are `vitest-json` and `fallow-json` only; the judgment crate stays empty and `judgment advise` is M5's optional advice; importers (dependency-cruiser, boundaries, Nx) are not built | Spec 9.3 and 21: readers follow the first adopter's need; Atlas tests run on vitest and the spec names Fallow as its duplicate-code instrument; D3 defers model-triggered required review and M5 stages optional advice | A needed reader is one bounded task later. Type, coverage, and mutation evidence requirements that Atlas's contracts name stay missing evidence until their reader is qualified; they are carried, never dropped |
| P10 | The experiment record is two numbers per task read from the workflow journal and audit: adopted findings at or above `major` on the first review round, and fix rounds consumed, plus the route that closed the task and an explicit row for every stalled, parked, or abandoned attempt. Denominator: every build and spike task in this plan. Baseline: the recorded runs of prior plans on this estate with Luna executors, which were not controlled; the honest claim at close is an observation, not a comparison result | Trey asked for the experiment; predeclaring the denominator keeps it from being read selectively afterward, and naming failed attempts keeps the all-task denominator from hiding them | If the journal lacks a field, the close report says so rather than estimating |
| P11 | The recorded harness run for before-decision delivery uses Claude Code (session-start and prompt-submit hooks), the harness whose hook documentation was read on 2026-09-17; Codex is not a required second adapter in this plan | One verified adapter is sufficient for M1 (spec 12.6, S7) | If Claude Code's hook behavior defeats delivery, M1 cannot close and S7 and the R14 comparison wait; switching adapters is a plan amendment plus fresh S7 qualification, not a swap |
| P12 | The frozen corpus in this plan holds Atlas plus the five open-source TypeScript shapes spec 17.3 names (a Next.js application, a pnpm monorepo with project references, a CommonJS library, a Vite library, a path-alias and package-exports repository), chosen by the W0.8 lane for shape and pinned with their dependency trees. A member that cannot be obtained is reported as an unresolved prerequisite, never dropped from a denominator | S1 and S6 measure over the corpus, and the parity matrix needs those shapes to cover the resolution features Atlas alone may not exercise | More corpus preparation in M0; if a chosen repository proves a poor fit, its replacement is a manifest change with a new corpus id |
| P13 | The gate's corpus stage runs public corpus members everywhere (fetched at their pinned commits by `scripts/corpus.sh fetch` and digest-checked) and private members where `WARRANT_CORPUS_DIR` holds them, printing `not run: private member unavailable` otherwise. Spec 17.6 requires the Atlas budgets measured in CI, which public GitHub CI cannot do under D4; that needs a private CI execution path (a Forgejo Actions runner on the estate or a self-hosted runner with the corpus directory), which is infrastructure Trey authorizes separately. Until it exists, every milestone close records `CI corpus job with Atlas: blocked prerequisite` beside the local gate result, and W2.G carries it to Trey by name; local gate runs are recorded as local runs, never as the CI claim | Spec 18.1 puts corpus smoke in the gate and 17.6 measures budgets in CI on the corpus; D4 restricts publication, not private execution | If Trey declines the runner, the Atlas CI requirement stays an open prerequisite through R14 and M3 planning must resolve it; nothing in this plan calls a local run CI |
| P14 | Every review panel seats Opus 5 `xhigh`, Sol `xhigh`, and GLM `high`, with Grok attacking. Sol is the same OpenAI family as the Astra executor, which the house rule on reviewer decorrelation forbids; Trey chose it deliberately (2026-09-19) to compare what each apex reviewer catches on the same diff and to learn whether same-family review is weaker. Each milestone review record tallies findings per reviewer seat | The panel keeps two families independent of the author, and the question is worth an answer | Same-family blind spots may pass a defect both Astra and Sol share; Opus and GLM are the check |
| P15 | An Opus 5 `high` session on Trey's personal subscription drives the build: renders and launches the workflow, handles parks and merges, runs the gate, and logs the run. Fable is the coordinator of record and the escalation layer: every checkpoint assigned `fable`, every adjudication whose findings the executor disputes or whose record carries an accepted major residue, every park, every deviation from the plan, and anything that would enter a review record as a ruling. The machine adjudicator seat is GLM first so the driver does not grade its own run. Gates that belong to Trey stay his | Trey's structure (2026-09-19): the driving is mechanical and the judgment is not; Fable stays reachable without doing the driving | If the escalation list is too wide the channel becomes a permission queue; if too narrow a bad fix ships; the list is revisited at W0.G |

## 5. Scope fence

Included: the Cargo workspace and gate; snapshot, inventory, program model, TypeScript and minimal Cargo integrations; policy language, query, census, context, propose, map; one harness adapter; S1, S3, S6, S7 with reports; obligations, findings, evidence binding for two adapters, four-fact advisory verdict, signals, profile parsing; rulings, signing glue, verification, exceptions, widening classifier, check receipts and the verifier; mutation testing and adversarial fixtures; the Atlas advisory dogfood and the early comparison; the human checkpoints named in section 6.

Excluded: `warrant gate` as an authoritative command and the D5 deployment; any account, service, runner, or infrastructure creation; evidence classes credited by a protected producer; MCP server; additional harness adapters; LSP; judgment backends and `judgment advise`; policy importers; incremental analysis and its performance budget; cross-tree evidence carry-over; instrument upgrade reports and `self-qualify`; VSA export; DSSE; Rust source references beyond Cargo package edges; publication of any Atlas artifact; GitHub pushes, tags, releases. Discovered work executes only when it blocks a task's acceptance; anything else is reported in `discovered` and becomes a plan amendment.

## 6. Gates and authority

Level 3 (multi-milestone, signing, external contracts). Human checkpoints, all assigned to Trey: W1.C1 (Atlas module and ownership declarations reviewed), W1.C2 (S7 sample, failure-cost criteria, output budget approved), W1.D10 (S1 decision), W2.C1 (comparison rubric and usefulness criteria approved), W2.G (R14: proceed, rethink, or mixed). Milestone gates W0.G and W1.G are oracle gates held by Fable: a decorrelated critical review of the milestone diff at its HEAD, with dispositions recorded, before the next milestone's tasks admit. Plan approval itself is Trey's; Astra reviews the plan first. Build authorization is separate from plan approval and is recorded in `docs/plans/live-state.md` when given.

## What I need from you to lock

Approved. Trey approved P1 to P13 and the scope fence and authorized the build on 2026-09-19, with P13's runner carried as a blocked prerequisite through R14 and P14 and P15 added in the same conversation.

After Astra's review is dispositioned: approve or amend P1 to P13 and the scope fence, and decide P13's open half: whether to authorize a private CI runner (Forgejo Actions on the estate, or a self-hosted runner holding the private corpus) so the Atlas corpus job runs in CI as spec 17.6 requires. Then say whether the build is authorized.

## Plan and routing

```toml plan
name = "warrant-v1-m0-m2"
repo = "../.."

[regimes.prose]
lanes = 1
min_families = 1
fix_rounds = 0
re_review = "none"
adjudicate = false
severity_floor = "major"
max_task_calls = 8

[regimes.standard]
lanes = 3
min_families = 3
fix_rounds = 1
re_review = "scoped"
adjudicate = true
severity_floor = "major"
max_task_calls = 12

[regimes.critical]
lanes = 3
min_families = 3
fix_rounds = 2
re_review = "scoped"
adjudicate = true
severity_floor = "major"
max_task_calls = 24
```

```toml routing
[executor]
timeout = 5400
routes = [{engine="codex", model="astra", effort="low", mode="work"}, {engine="codex", model="sol", effort="xhigh", mode="work"}]
[fixer]
routes = [{engine="codex", model="astra", effort="low", mode="work"}, {engine="codex", model="sol", effort="high", mode="work"}]
[integrator]
routes = [{engine="codex", model="luna", effort="medium", mode="work"}, {engine="omp", model="gemini", effort="high", mode="work"}]
[reviewer]
routes = [{engine="claude", model="claude-opus-5", effort="xhigh", mode="work"}, {engine="codex", model="sol", effort="xhigh", mode="work"}, {engine="omp", model="glm", effort="high", mode="work"}]
[attacker]
routes = [{engine="cursor", model="cursor-grok-4.6-xhigh", effort="high", mode="work"}, {engine="grok", model="grok-4.6", effort="high", mode="work"}]
[adjudicator]
routes = [{engine="omp", model="glm", effort="high", mode="work"}, {engine="claude", model="claude-opus-5", effort="xhigh", mode="work"}]
[verifier]
routes = [{engine="codex", model="luna", effort="xhigh", mode="work"}, {engine="omp", model="gemini", effort="high", mode="work"}]
```

Routing, in words. Every build and spike task executes on Astra at `low` first (P2); a bounded retry rotates to Sol at `xhigh`, which is the same OpenAI family, so the reviewer and adjudicator pools stay decorrelated from the executor whichever route ran. The fixer is the same pair, because the fix belongs with the lane that holds the context. Reviewer personas are assigned round-robin over the reviewer pool, so `refuter` lands on Opus 5, `conventions` on Cursor Grok 4.6, and `test-skeptic` on GLM 5.3; no OpenAI model reviews the OpenAI executor. Critical tasks swap `conventions` for `attacker`, which draws from the attacker pool at its own slot (Grok Build there, Cursor Grok 4.6 as the other route), so the three seats stay Anthropic, xAI, and Zhipu. Prose tasks carry one `spec-fidelity` seat on Opus 5. Integration is mechanical and runs on Luna at `medium` with Gemini 3.8 Flash behind it; verify fan-outs run Luna at `xhigh` per the 2026-08-27 ruling. The adjudicator is Opus 5 at `xhigh` with GLM behind it, so neither the executor's family nor the coordinator's own session adjudicates. The executor role timeout is a safety bound for a low-effort apex lane on a crate-sized task, not an estimate. `plan-lint --check-routes` verifies these aliases against Delegate's live table before compilation.

## Interfaces

Names shared by sibling lanes. A lane uses these exactly; a lane that needs a name not listed here reports it in `discovered` rather than inventing one.

### Workspace and crates

Cargo workspace at the repository root, edition 2024, `rust-toolchain.toml` pinning stable, MSRV per spec 19.1. Packages: `warrant-core`, `warrant-snapshot`, `warrant-inventory`, `warrant-model`, `warrant-lang-ts`, `warrant-lang-rust`, `warrant-evidence`, `warrant-authority`, `warrant-judgment`, `warrant-census`, `warrant-render`, `warrant-cli`, each at `crates/<name without prefix>/`. The binary is `warrant`, built by `warrant-cli`. Dependency direction is spec 3.1 and is checked by `scripts/deps.sh` (reads `cargo metadata`, exits 1 on a forbidden edge; `scripts/deps.sh --self-test` plants a forbidden edge in a temporary copy and must print `self-test: violation detected`). W0.2 rechecks each section 19 candidate the plan's slice needs (current version, license under `cargo deny`, runtime identity) and records the result in `docs/research/2026-09-dependency-recheck.md`, then adds the accepted pins to `[workspace.dependencies]` and to the manifest of each crate the spec assigns them to, so later lanes rarely touch `Cargo.toml`; optional and deferred entries are left out until their adoption task. A lane that must change a manifest declares that file in `discovered` and the coordinator re-sequences.

`crates/core/src/lib.rs` declares this module tree at W0.2, each as an existing file or directory: `nouns` (snapshot manifest, inventory entry and summary, capability report, error document, tool identity), `manifest` (`warrant.yaml`), `policy` (contract types, declarations, selectors, compiler, canonical digest, structural lint), `eval` (obligations, outcomes, snapshot-dependent lint), `pattern` (ast-grep rules and registries), `evidence` (evidence kinds and evidence obligations), `findings` (explanations, grouping, identity), `retrieval` (context and propose), `verdict` (facets, acceptance, exit mapping, human rendering data), `receipt` (statement, canonical bytes, verifier), `signals`, `profile`, `classify` (widening classifier), `fault` (fault-injection hooks, test builds only), `schema` (schemars generation). Later tasks own subdirectories of this tree and never `lib.rs`.

The receipt verifier lives in `core` (spec 3.1) and `core` depends on nothing, so `core::receipt::verify` owns the step order, the binding comparisons, and the result derivation of spec 10.5, and takes three trait objects for everything it cannot do alone: `Recompute` (does the tree exist; the policy blobs and compiled digest at that tree; the inventory digest at that tree; the model digest at that tree with the pinned integrations; the instrument lock bytes at that tree) implemented in `warrant-cli` over `snapshot`, `inventory`, `model`, `evidence`, and the `lang-*` crates; `Artifacts` (load a test receipt, report file, or ruling artifact from the local evidence store by its recorded digest, returning the bytes or absent) implemented in `warrant-cli` over `warrant-evidence` and `warrant-authority`; and `RulingVerifier` (ruling signature, chain, supersession, key validity, producer signature) implemented in `warrant-authority`. Steps 1 and 8 run in `core` on the receipt's own bytes; steps 2 to 6 call `Recompute` (step 6 compares the recorded instrument identities with the lock read at the subject tree, never with the receipt's own copy); step 7 loads each required artifact through `Artifacts`, marks an absent one unchecked, checks digests, subjects, and execution bindings in `core`, and passes rulings to `RulingVerifier`; step 9 calls `RulingVerifier`. Without `--recompute`, `Recompute` is not called for steps 4 and 5 and those bindings are reported unverified. A receipt conformance case must traverse the real composition with `--recompute`, not a stubbed `Recompute`. The widening classifier is `core::classify` and reads canonical policy only.

Every emitted document is a `serde` type with a `schemars` derive; `warrant schema <name>` prints it and `scripts/stages/40-schema.sh` regenerates `schemas/` and fails on a diff. W0.2 pre-declares one file per document under `crates/core/src/schema/<name>.rs`, each a stub that reports `not-implemented` until the task that delivers the document replaces it and checks in `schemas/<name>.json`; the schema stage lists implemented and stub documents, and the acceptance script requires zero stubs among the documents this plan delivers. Schema names are the spec's `schema_version` prefixes: `warrant.snapshot`, `warrant.inventory`, `warrant.capabilities`, `warrant.commands`, `warrant.manifest`, `warrant.policy`, `warrant.effective-policy`, `warrant.obligations`, `warrant.finding`, `warrant.verdict`, `warrant.context`, `warrant.propose`, `warrant.map`, `warrant.census`, `warrant.test-receipt`, `warrant.evidence`, `warrant.instruments`, `warrant.ruling`, `warrant.receipt`, `warrant.verify`, `warrant.diff`, `warrant.error`, `warrant.hook`, `warrant.profile`. Each producing task owns its `crates/core/src/schema/<name>.rs` and `schemas/<name>.json`. `warrant.commands` is the document `warrant capabilities` emits (the command list with implemented and stub status, the implemented schema names, and adapters); it was added by ruling F36 after W0.7's review found the command emitting `warrant.capabilities/1` while violating that schema, which describes the per-integration `CapabilityReport` of spec 6.1. W0.7 owns it and it is in scope at M0. The same pre-declaration device applies inside the other crates: `warrant-model` declares `declared` and `query` at W1.1, `warrant-lang-ts` declares `parity`, `frameworks`, and `references` at W1.2, `warrant-snapshot` declares `native` at W0.3, and `warrant-authority` declares `batch` at W2.7, each as a stub module already called from the crate's real composition path (the model build calls `frameworks` and `references` for every unit and they return empty results; `query` returns not-implemented; `native` returns unsupported; `batch` returns an empty batch), so the owning task replaces the stub body and touches no composition file. The task that declares a stub owns the wiring; the task that fills it owns only the module.

### Gate, stages, acceptance

`scripts/gate.sh` prints tool versions first (rustc, cargo, clippy, cargo-deny, cargo-mutants when present, git, node when present), then asserts that every stage named in `scripts/stages/REQUIRED` exists and is executable, then runs every executable under `scripts/stages/` in lexical order. Stage files: `10-fmt.sh`, `20-clippy.sh`, `25-deny.sh`, `30-test.sh`, `35-deps.sh`, `40-schema.sh`, `45-corpus.sh`, `50-conformance.sh`, `55-budget.sh`, `60-mutants.sh`. A task that adds a stage adds its name to `REQUIRED` in the same change, so a deleted or non-executable stage fails the gate instead of vanishing. The corpus stage follows P13 and, from M2, asserts the applicable spec 17.6 budgets: cold full evaluation of the Atlas member under twenty seconds and one gibibyte, `warrant verify` without `--recompute` under one second, each measured and failed on regression. `scripts/budget.sh` holds per-crate source-line budgets (initial alarms: core 8000, lang-ts 6000, authority 3000, cli 4000, every other crate 2500); raising one requires an architecture note in the commit body (HC16).

`tests/acceptance.sh` runs `scripts/gate.sh` and then every executable under `tests/acceptance.d/` in lexical order; each item is one numbered demo from the goal lock and prints `ACCEPT <n>` or `REFUSE <n> <reason>`. The coordinator writes `tests/acceptance.sh`, the `bin/delegate-audit` exec shim, and `docs/acceptance/build-start.txt` (the ISO time the build started) before launch; no lane owns them. The acceptance script reads `tests/acceptance.d/REQUIRED`, a list of item numbers that must exist and be executable at the current point of the build; a task that adds an item appends its number in the same change, so a listed item that is missing, not executable, or prints `REFUSE` fails the script. `tests/acceptance.sh --final`, which the coordinator runs at every milestone close and at plan close, additionally requires items `02` and `03` for M0 close, `02` to `05` for M1 close, and `01` to `10` for M2 close (item `01`, the gate item, is created by W2.10), the corpus stage output reporting the Atlas member ran, and the schema stage reporting zero stub documents among the documents in scope at that milestone. The stages print marker lines that `--final` reads: `scripts/stages/40-schema.sh` prints one line per pre-declared document, `schema: <name> implemented` or `schema: <name> stub`, in a stable order, and may follow them with a `schema: stub-documents <count>` summary; `scripts/stages/45-corpus.sh` prints one line per corpus member, `corpus: <name> ran` or `corpus: <name> not run: private member unavailable`. `--final` carries the in-scope document list for its milestone and fails when an in-scope document is reported `stub` or is absent from the output, so an unmarked stage is a red result and never a silent pass; a task that edits a stage preserves its marker lines. A document is in scope at a milestone when its owning task is at or before that milestone: at M0 `warrant.snapshot`, `warrant.inventory`, `warrant.capabilities`, `warrant.manifest`, and `warrant.error`, all owned by W0.2, and `warrant.commands` (W0.7); M1 adds `warrant.instruments` (W1.4), `warrant.policy` and `warrant.effective-policy` (W1.7), `warrant.census` (W1.10), `warrant.context` and `warrant.propose` (W1.11), `warrant.map` (W1.12), and `warrant.hook` (W1.13); M2 adds `warrant.obligations` (W2.1), `warrant.finding` (W2.3), `warrant.evidence` and `warrant.test-receipt` (W2.4), `warrant.verdict` and `warrant.receipt` (W2.5), `warrant.profile` (W2.6), `warrant.ruling` (W2.7), `warrant.diff` (W2.8), and `warrant.verify` (W2.9). Task verify rows call the plain form.

### CLI contract

Command names are exactly spec section 12.1. `warrant-cli` registers every command at W0.7, each in `crates/cli/src/commands/<name>.rs` (`snapshot`, `inventory`, `model`, `query`, `context`, `propose`, `check`, `gate`, `explain`, `verify`, `policy`, `rule`, `attest`, `evidence`, `instrument`, `census`, `map`, `serve`, `hook`, `selftest`, `judgment`, `schema`, `capabilities`); an unimplemented command exits 2 with a `warrant.error` document whose code is `not-implemented`. Later tasks own only their command file. Off-terminal output is JSON with no ANSI (HC11); `--format json|human`; exit codes per spec 10.2: 0 clean, 1 blocked, 2 could-not-evaluate, 3 internal, 130 and 143 on signal. `warrant capabilities` lists implemented commands and adapters.

Conformance: fixtures under `tests/conformance/<area>/<case>/` with `repo/` (a fixture repository, initialized as git by the runner from a checked-in tree), `warrant/` policy where the case needs it, `expect.json` (semantic expectations: facet values, finding contract and obligation and subject, exit code, inventory classes), and `README.md` naming the hard control the case proves and its positive, negative, and break-the-control variants. The runner is `crates/cli/tests/conformance.rs`, invoked as `cargo test --locked -p warrant-cli --test conformance -- <area>`; it builds the `warrant` binary, runs the case's commands, and compares `expect.json` semantically, never byte-for-byte. Areas in this plan: `inventory`, `snapshot`, `model`, `policy`, `context`, `claims`, `identity`, `verdict`, `authority`, `execution`, `combination`, `census`, `adapters`, `offline`, `bounded`, `adversarial`, `qualification`.

Corpus: `tests/corpus/manifest.yaml` lists each corpus repository with name, source, pinned commit, tree sha256, dependency bundle sha256, publishability, and the frameworks it uses; `WARRANT_CORPUS_DIR` names the local directory holding private tarballs (P4). Members: Atlas (private) and the five public shapes of P12. `scripts/corpus.sh verify` checks every digest and exits 1 on a missing or mismatched artifact; `scripts/corpus.sh fetch <name>` obtains a public member at its pinned commit with its lockfile-installed dependencies and checks the digests; `scripts/corpus.sh unpack <name> <dir>` materializes one. The Atlas pin record is `docs/corpus/2026-09-19-atlas-pin.md`; each public member has a record beside it.

### Spike and report paths

Spike reports go to `docs/spikes/<date>-<spike>.md` with a status line, the question, the corpus used, measurements, and the exit criterion's result stated as met or not met. S3 code lives behind a `native-capture` feature in `crates/snapshot`; S1 code lives under `spikes/s1/` and is not a workspace member; S6 is `warrant instrument qualify lang-ts` and its matrix report; S7's protocol, sample, and report live under `docs/spikes/` with the redacted sample in `WARRANT_CORPUS_DIR` and its digest in the report.

### Milestone reviews and the experiment record

A milestone gate (W0.G, W1.G, W2.R, W2.R2) is closed by the coordinator after a critical-regime review of the milestone diff at its HEAD: three reviewers from three families, findings dispositioned in `docs/plans/reviews/<date>-<milestone>.md`, adopted majors fixed before the next milestone admits. The experiment record (P10) is `docs/acceptance/2026-M2-executor-experiment.md`: one row per build and spike task with first-round major findings, fix rounds, and the executor route that closed it, read from the workflow journal and `bin/delegate-audit`.

## Tasks

```toml task
id = "W0.1"
title = "Create the Cargo workspace, gate, and acceptance scaffolding"
delivers = "A building workspace of twelve empty crates named per Interfaces, the gate script with its stage directory and the REQUIRED stage inventory, per-crate budgets, the dependency-direction check with its self-test, the tests/acceptance.d directory with its README stating the ACCEPT and REFUSE convention and an empty REQUIRED list, the CI workflow with a stable job and a pinned-MSRV job both running the gate, cargo-deny configuration, and AGENTS.md section 'Conventions the build will follow' replaced by the real layout"
kind = "build"
blocked_by = []
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "house-bar"]
owned_files = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "deny.toml", "crates", "scripts", "tests/acceptance.d", ".github", "AGENTS.md", "README.md", ".gitignore"]
invariants = ["scripts/deps.sh fails on a forbidden crate edge and its self-test proves the detector binds", "scripts/gate.sh fails when a stage named in REQUIRED is missing or not executable, and fails when any stage fails"]
acceptance = "cargo build --locked succeeds for all twelve crates; scripts/gate.sh passes with the fmt, clippy, deny, test, deps, and budget stages; scripts/deps.sh --self-test prints self-test: violation detected; the coordinator-provided tests/acceptance.sh runs the gate and, with an empty REQUIRED list, reports no acceptance items and exits 0; AGENTS.md describes the real layout; the lane records one gate run with a REQUIRED stage renamed away failing. Green does not prove any Warrant behavior or that CI executed; the crates are empty and CI execution is checked at W0.G."
verify = [{run = "cargo build --locked --workspace", expect = "exit 0"}, {run = "scripts/deps.sh --self-test", expect = "self-test: violation detected"}, {run = "scripts/gate.sh", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs", "network"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.2"
title = "Define the core nouns, manifest, and schema generation"
delivers = "warrant-core with the pre-declared module tree including retrieval and fault, one schema stub per emitted document under crates/core/src/schema, serde and schemars types for the snapshot manifest, inventory entry and summary, capability report, error document, tool identity, and the warrant.yaml manifest with its documented defaults; schemas/ generated and checked in for the implemented documents; the schema stage reporting implemented and stub documents and printing its marker lines per Interfaces; the section 19 dependency recheck recorded in docs/research/2026-09-dependency-recheck.md and only the accepted pins this plan needs added to the workspace and crate manifests"
kind = "build"
blocked_by = ["W0.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core", "schemas", "scripts/stages/40-schema.sh", "docs/research", "Cargo.toml", "Cargo.lock", "crates/snapshot/Cargo.toml", "crates/inventory/Cargo.toml", "crates/model/Cargo.toml", "crates/lang-ts/Cargo.toml", "crates/lang-rust/Cargo.toml", "crates/evidence/Cargo.toml", "crates/authority/Cargo.toml", "crates/judgment/Cargo.toml", "crates/census/Cargo.toml", "crates/render/Cargo.toml", "crates/cli/Cargo.toml"]
invariants = ["Every emitted document type carries schema_version and regenerates its checked-in schema byte-identically", "The manifest parser rejects an unknown field and an unknown class name with a typed error naming the field"]
acceptance = "cargo test --locked -p warrant-core passes with tests for manifest parsing, defaults, unknown-field rejection, and schema round-trip; scripts/stages/40-schema.sh passes and fails when one schema file is hand-edited; the module tree in Interfaces exists with a doc comment per module saying what it owns and must not know; the recheck record names each accepted pin's verified version and license and each deferred entry. Green does not prove any schema matches the spec's examples semantically beyond the fields the tests name."
verify = [{run = "cargo test --locked -p warrant-core", expect = "exit 0"}, {run = "scripts/stages/40-schema.sh", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs", "network"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.8"
title = "Pin the corpus: Atlas plus the five public shapes"
delivers = "docs/corpus/2026-09-19-atlas-pin.md naming the pinned Atlas commit, the actual compensation and undo symbols, their tests, and the transports section 1.7 refers to; tests/corpus/manifest.yaml with the Atlas entry and the five public shapes of P12, each pinned with its dependency tree, digests, publishability, and the frameworks it uses; scripts/corpus.sh verify, fetch, and unpack; the Atlas tarball and dependency bundle written to WARRANT_CORPUS_DIR; a pin record per public member under docs/corpus naming why that repository was chosen for its shape; recorded dependency and instrument inputs (git, node, npm, typescript versions observed)"
kind = "build"
blocked_by = ["W0.1"]
role = "executor"
persona = "repro-first-writer"
skills = ["research", "write-human"]
owned_files = ["docs/corpus", "tests/corpus", "scripts/corpus.sh"]
invariants = ["No home-directory path, internal hostname, or credential appears in any committed corpus file", "scripts/corpus.sh verify fails when a tarball digest does not match the manifest"]
acceptance = "The pin record names the commit, lists the real symbols under packages/core/src with paths, and separates verified facts from section 1.7's illustrative names; scripts/corpus.sh verify exits 0 against WARRANT_CORPUS_DIR and exits 1 after one byte of the tarball is changed; scripts/corpus.sh fetch obtains each public member and its digests match; the manifest records which frameworks Atlas uses so W1.8 knows whether an adapter is needed; rg for the home directory prefix and tailnet hostnames over the committed files finds nothing. Green does not prove any member is a complete build environment; that is checked when S6 first unpacks them. A member that cannot be obtained is reported as an unresolved prerequisite in discovered, never omitted."
verify = [{run = "scripts/corpus.sh verify", expect = "exit 0"}, {run = "sh -c 'rg -n \"/home/|\\.ts\\.net|100\\.[0-9]+\\.\" docs/corpus tests/corpus scripts/corpus.sh; test $? -eq 1'", expect = "exit 0"}]
reversibility = "reversible"
effects = ["docs", "code", "filesystem"]

[review]
tier = "standard"
tier_reason = "the only filesystem effect is writing a tarball into an empty corpus directory the lane creates under WARRANT_CORPUS_DIR; it is reversible by deletion and touches nothing else"
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W0.3"
title = "Implement snapshot capture, identity, and verified reads"
delivers = "warrant-snapshot: commit, index, worktree, and supplied-tree kinds; identity as object-format plus tree id; repository identity from the root commit; worktree capture through a temporary index that never touches the real index; exclusion accounting for ignored, oversize, external-symlink, submodule, and case-collision entries; blob-verified reads with one retake and the snapshot-unstable error; every worktree read routed through one function with a test-build counter of read attempts; a stub native module pre-declared for W0.4 and called from the hashing path, returning unsupported so the portable path runs; the manifest document; input errors for unmerged index, missing tree, and non-repository"
kind = "build"
blocked_by = ["W0.2"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/snapshot"]
consumes = ["crates/core/src/nouns", "schemas"]
invariants = ["An index or commit snapshot makes zero worktree read attempts, counted at the single read function in test builds", "A worktree read whose blob no longer matches the captured id fails closed after one retake with snapshot-unstable and no artifact"]
acceptance = "cargo test --locked -p warrant-snapshot passes with tests for each kind, for an index and a commit snapshot completing with the worktree read counter at zero while the worktree is dirty, for the temporary-index capture leaving the real index byte-identical, for tracked-but-ignored files remaining present, for a mid-capture rewrite producing snapshot-unstable, and for each input error's exit code; the unstable test was observed failing before the retake logic existed. Green does not prove parity with git write-tree for the native path; that is S3."
verify = [{run = "cargo test --locked -p warrant-snapshot", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.5"
title = "Implement the inventory classifier and completeness accounting"
delivers = "warrant-inventory: one entry per path and per deliberate exclusion; classes per spec 5.1 with class defaults from integrations and explicit overrides that must name the default they replace; ownership separate from class; unit discovery from package.json workspaces, pnpm-workspace.yaml, nearest tsconfig, and Cargo workspaces; entrypoints from package.json fields and warrant.yaml declarations; generated and vendored provenance with producers; warrant inventory --verify-generated re-running producers marked reproducible in a temporary directory and comparing blob ids, reporting generated-drift and generated-absent; the completeness summary with unread, unowned_source, unknown, and submodule lists; the inventory digest"
kind = "build"
blocked_by = ["W0.2"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/inventory"]
consumes = ["crates/core/src/nouns", "crates/core/src/manifest", "schemas"]
invariants = ["A first-party file outside every module selector is listed under unowned_source, never silently classified", "Overlapping module selectors and conflicting explicit class assignments are lint errors, never resolved by order"]
acceptance = "cargo test --locked -p warrant-inventory passes with tests for each class default, for a colocated test keeping class test inside its owning module, for unowned_source, for overlap and conflict errors, for unit discovery on a monorepo tree, for --verify-generated reporting drift and absence against a fixture producer, and for the summary counts equalling the entry list; the overlap test was observed failing before the check existed. Green does not prove framework entrypoints; those adapters are M1 instruments."
verify = [{run = "cargo test --locked -p warrant-inventory", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.4"
title = "Spike S3: native worktree and index hashing with gix"
delivers = "Creates crates/snapshot/src/native.rs, crates/snapshot/tests/native.rs, and docs/spikes/2026-09-S3-native-capture.md, the report answering whether gix reproduces git write-tree tree ids over the corpus including untracked, modified, ignored, symlinked, and executable files; native capture behind the native-capture feature in warrant-snapshot; the feature enabled by default only if the exit criterion is met, otherwise left off with the reason recorded; the decision it settles is whether native capture ships on by default"
kind = "spike"
blocked_by = ["W0.3"]
role = "executor"
persona = "repro-first-writer"
skills = ["rust-engineer", "research"]
owned_files = ["crates/snapshot/src/native.rs", "crates/snapshot/tests/native.rs", "crates/snapshot/Cargo.toml", "docs/spikes/2026-09-S3-native-capture.md"]
invariants = ["Identical tree ids between native capture and git write-tree over every S3 case, or the feature stays off"]
acceptance = "The report lists every corpus case with both tree ids and states met or not met; the feature default matches the result; cargo test --locked -p warrant-snapshot --features native-capture passes when enabled. Green does not prove parity on filesystems other than the devbox's."
verify = [{run = "cargo test --locked -p warrant-snapshot --all-features", expect = "exit 0"}, {run = "sh -c 'rg -q \"^Status: met\" docs/spikes/2026-09-S3-native-capture.md && test \"$(rg -c \"\\| measured match \\|\" docs/spikes/2026-09-S3-native-capture.md)\" -eq 6'", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W0.7"
title = "Build the CLI shell with snapshot, inventory, schema, and capabilities"
delivers = "warrant-cli: clap command tree registering every section 12.1 command name as its own file with not-implemented stubs; implemented snapshot, inventory, schema, and capabilities; the output contract (JSON off-terminal, --format, no ANSI, NDJSON for lists, cursors with truncated and next_cursor); exit codes per spec 10.2; cooperative SIGINT and SIGTERM with no partial artifact; the cache layout under XDG_CACHE_HOME with atomic writes"
kind = "build"
blocked_by = ["W0.3", "W0.5"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "rust-agent-cli", "tdd"]
owned_files = ["crates/cli"]
invariants = ["Machine output never contains an ANSI escape or progress text", "A cache artifact is readable only after its atomic rename; an interrupted write leaves no partial file"]
acceptance = "cargo test --locked -p warrant-cli passes with tests that pipe stdout and assert no ESC byte, that every stub exits 2 with a warrant.error document, that snapshot and inventory print schema-valid JSON, that SIGTERM during a capture leaves the cache directory without a partial artifact and exits 143, and that --format human renders the same data. Green does not prove any analysis; the model and policy commands are stubs."
verify = [{run = "cargo test --locked -p warrant-cli", expect = "exit 0"}, {run = "sh -c 'cargo run -q --bin warrant -- capabilities --limit 5 | python3 -c \"import json,sys; d=json.load(sys.stdin); s=json.load(open(\\\"schemas/warrant.commands.json\\\")); assert d[\\\"schema_version\\\"]==\\\"warrant.commands/1\\\"; assert s[\\\"additionalProperties\\\"] is False and set(s[\\\"required\\\"])<=set(d)<=set(s[\\\"properties\\\"]); assert d[\\\"truncated\\\"] and d[\\\"next_cursor\\\"]==5 and len(d[\\\"commands\\\"])==5 and d[\\\"total\\\"]>5; assert \\\"snapshot\\\" in d[\\\"implemented\\\"] and \\\"warrant.commands\\\" in d[\\\"schemas\\\"]\"'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.6"
title = "Build the conformance harness and seed the inventory and snapshot fixtures"
delivers = "crates/cli/tests/conformance.rs runner per Interfaces; tests/conformance/inventory and tests/conformance/snapshot cases covering the section 5.7 proofs available at M0 (unowned first-party file, colocated test class, overlap rejected at manifest load, generated-drift and generated-absent) and section 4's exclusions, each with positive, negative, and break-the-control variants, with the registry-removal and unreadable-file proofs assigned by name to W2.2 and W2.1; the conformance stage; the corpus stage per P13, printing its marker lines per Interfaces, with semantic snapshot and inventory expectations for every corpus member, both new stages added to scripts/stages/REQUIRED and items 02 and 03 added to tests/acceptance.d/REQUIRED; Specgate fixture projects ported as seeds where they exercise inventory or snapshot behavior; acceptance.d items for demos 2 and 3"
kind = "build"
blocked_by = ["W0.7", "W0.8"]
role = "executor"
persona = "repro-first-writer"
skills = ["rust-engineer", "tdd", "house-bar"]
owned_files = ["crates/cli/tests", "tests/conformance", "scripts/stages/50-conformance.sh", "scripts/stages/45-corpus.sh", "scripts/stages/REQUIRED", "tests/acceptance.d/02-snapshot-index.sh", "tests/acceptance.d/03-inventory-unowned.sh", "tests/acceptance.d/REQUIRED"]
consumes = ["crates/cli/src/main.rs", "tests/corpus/manifest.yaml", "scripts/corpus.sh"]
invariants = ["Every case's break-the-control variant fails the runner when the guarded control is disabled", "The runner compares expect.json semantically and never byte-for-byte"]
acceptance = "cargo test --locked -p warrant-cli --test conformance -- inventory and -- snapshot pass; disabling the unowned-source check in a scratch build makes the inventory break-the-control case fail; the two acceptance.d items print ACCEPT; the corpus stage runs the public members and Atlas and fails when a member's expected inventory class count differs; the Specgate seeds are listed in tests/conformance/README.md with what each proves. Green does not prove the harness covers hard controls that later milestones add."
verify = [{run = "cargo test --locked -p warrant-cli --test conformance -- inventory", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- snapshot", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 3"}]
reversibility = "reversible"
effects = ["code", "network"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W0.G"
title = "Close M0 after the milestone review"
delivers = "A critical-regime review of the M0 diff at its HEAD with every finding dispositioned in docs/plans/reviews/, adopted majors fixed, and M0's exit criteria from spec section 21 checked against the conformance results"
kind = "checkpoint"
blocked_by = ["W0.6", "W0.4"]
acceptance = "docs/plans/reviews/<date>-M0.md exists with three reviewers from three families, dispositions for every finding, and the M0 exit statement: staged and committed reads ignore unrelated worktree bytes, tracked ignored files remain present, missing input is explicit, colocated tests keep their class, S3 resolved; CI observed executing the gate on stable and MSRV, or the gate reports that prerequisite blocked."
assignee = "fable"
gate = "judgment"
because = "the spec makes every milestone diff a critical review and M1 tasks must not build on an unreviewed foundation"
```

```toml task
id = "W1.1"
title = "Implement the program model store"
delivers = "warrant-model: the SQLite schema from spec 6.2 with all tables and materialized views, streaming writers that build the model as integrations report, the model digest over a canonical dump, read-only opening with an authorizer that rejects anything but SELECT, row and byte caps with truncated and continuation, stub modules declared and query pre-declared for W1.8 and W1.9, each called from the model build path with empty results, and schemas/model.md documenting the schema for warrant query --schema"
kind = "build"
blocked_by = ["W0.G"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/model", "schemas/model.md"]
invariants = ["Equal semantic rows produce an equal model digest regardless of insertion order or timestamps", "A query connection cannot execute INSERT, UPDATE, DELETE, ATTACH, or PRAGMA writes"]
acceptance = "cargo test --locked -p warrant-model passes with tests for every table and view, for digest stability under shuffled insertion, for the authorizer rejecting each write statement kind, and for caps setting truncated with a continuation hint; the authorizer test was observed failing with the authorizer removed. Green does not prove any language integration populates the model correctly."
verify = [{run = "cargo test --locked -p warrant-model", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.7"
title = "Implement the policy language, compiler, and canonical digest"
delivers = "crates/core/src/policy: contract types for all nine kinds with common fields and documented defaults, declarations, typed selectors, the claim vocabulary and its fixed claim-to-capability mapping, the compiler producing an effective policy with a policy_digest that excludes placement and formatting, structural lint for unique ids, overrides authority, migration expiry, and enforcement-unsupported; warrant policy compile and the structural half of warrant policy lint; schemas for policy and effective-policy replacing their stubs"
kind = "build"
blocked_by = ["W0.G"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core/src/policy", "crates/cli/src/commands/policy.rs", "tests/conformance/policy", "crates/core/src/schema/policy.rs", "crates/core/src/schema/effective-policy.rs", "schemas/warrant.policy.json", "schemas/warrant.effective-policy.json"]
invariants = ["Reordering contracts across files or reformatting YAML leaves the policy digest unchanged", "A claim whose required capabilities a user lowers by editing a field is rejected at compile"]
acceptance = "cargo test --locked -p warrant-core policy passes; the policy conformance area passes with a digest-stability case (file move, reorder, reformat), a conflict case per spec 7.6 class that compiles without a snapshot, and an enforcement-unsupported case; warrant policy compile prints the effective policy with its digest. Green does not prove snapshot-dependent lint (empty selectors, drift), which W2.1 owns."
verify = [{run = "cargo test --locked -p warrant-core policy", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- policy", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.2"
title = "Implement TypeScript parsing, bindings, and the capability report"
delivers = "Creates warrant-lang-ts with discover and analyze over oxc: units from the inventory's tsconfig and workspace discovery, per-file symbols, exported bindings, import and re-export bindings with type-only flags, in-file references, dynamic-load sites, every unsupported construct with its reason, entrypoints from package.json and test-file conventions, and a capability report that names symbol_level binding, resolution_authority unqualified, and the supports and unsupported lists; warrant model building the model from the integrations with every edge unresolved pending W1.3 and every unsupported construct recorded; stub modules parity, frameworks, and references pre-declared for W1.4, W1.8, and W1.16 and already called from the analyzer's composition path with empty results, so those tasks replace module bodies only; Specgate's parser code ported behind the integration contract where it fits"
kind = "build"
blocked_by = ["W1.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/lang-ts", "crates/cli/src/commands/model.rs", "tests/conformance/model"]
invariants = ["A non-literal dynamic import is recorded as unsupported with dynamic-nonliteral, never resolved by guess", "The capability report never claims a construct the analyzer did not observe in tests"]
acceptance = "cargo test --locked -p warrant-lang-ts passes; the model conformance area passes with cases for ESM import, re-export chains, type-only imports, literal require, literal and non-literal dynamic import, and an unsupported construct; every entry in the report's supports list has a fixture that exercises it. Green does not prove resolution; edges are unresolved until W1.3."
verify = [{run = "cargo test --locked -p warrant-lang-ts", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- model", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.6"
title = "Implement the minimal Cargo integration"
delivers = "warrant-lang-rust: units and targets from pinned cargo metadata output with target and feature selection, resolved package-dependency edges with basis visibility, a capability report naming native build-system authority for those edges only, symbol_level none, and unsupported Rust-internal references; the model for Warrant's own workspace populated so its crate direction is queryable"
kind = "build"
blocked_by = ["W1.1", "W1.2"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/lang-rust"]
consumes = ["crates/model", "crates/cli/src/commands/model.rs"]
invariants = ["The Cargo integration claims authority only for package edges and reports every other reference kind as unsupported"]
acceptance = "cargo test --locked -p warrant-lang-rust passes with tests over a fixture workspace and over Warrant's own metadata; warrant model on this repository lists twelve units and the section 3.1 edges. Green does not prove any Rust use-graph or macro behavior."
verify = [{run = "cargo test --locked -p warrant-lang-rust", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.3"
title = "Implement TypeScript module resolution and observed interfaces"
delivers = "Adds resolution through oxc_resolver under each unit's configuration: tsconfig paths, package exports and imports, condition names derived per importer format, project references, the baseUrl fallback disabled for TypeScript 6 and later configurations; the resolve operation returning target file, external package, or unresolved with reason; module_edges, symbol_consumers, and module_cycles populated; observed interfaces per module; warrant model --capabilities"
kind = "build"
blocked_by = ["W1.2"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/lang-ts", "crates/cli/src/commands/model.rs", "tests/conformance/model"]
invariants = ["An unresolved edge is stored with its reason and counted in the analysis facet inputs, never dropped"]
acceptance = "cargo test --locked -p warrant-lang-ts passes; model conformance cases cover paths aliases, package exports with conditions, project references, a package with no matching export condition (unresolved with reason), and a cycle; warrant model --capabilities prints the report with resolution_authority unqualified. Green does not prove parity with tsc; that is S6."
verify = [{run = "cargo test --locked -p warrant-lang-ts", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- model", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.8"
title = "Populate declared facts and registry entrypoints into the model"
delivers = "Creates crates/model/src/declared and crates/core/src/pattern/registry.rs: declared modules, interfaces, external consumers, stores, effects, aliases, and required-evidence rows written into the model from the effective policy so query and context answer from them with basis declared; registry declarations matched as ast-grep patterns to produce entrypoints of kind registry with basis observed; the framework adapters the corpus manifest records as used by any member (at least Next.js file-system routes, given P12), each a versioned instrument under crates/lang-ts/src/frameworks enabled per manifest, with its recognition rules qualified against fixtures and the enabled member; declared loaders producing config_reference edges and config-referenced entrypoints with declared provenance; the model conformance area extended with declared-versus-observed cases"
kind = "build"
blocked_by = ["W1.3", "W1.7"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/model/src/declared", "crates/core/src/pattern/registry.rs", "crates/lang-ts/src/frameworks", "tests/conformance/model/declared", "crates/core/src/pattern/mod.rs"]
consumes = ["crates/core/src/policy", "crates/lang-ts", "crates/model"]
invariants = ["A declared fact never acquires an observed basis and an observed fact never acquires a declared one; both may exist for one subject and both are returned", "A registry pattern that matches nothing yields a drift row on the registry, never silent absence", "A framework adapter recognizes entrypoints only for the member whose manifest enables it; no framework knowledge lives in core"]
acceptance = "cargo test --locked -p warrant-model declared passes; model conformance cases cover a declared external consumer with no observed import (returned with basis declared), a declared interface whose observed exports differ (both rows), a registry with two matched symbols and one with none (drift), a Next.js route file recognized as a framework-route entrypoint only when the adapter is enabled, and a declared loader whose configuration names a file (config-referenced entrypoint and config_reference edge). Green does not prove obligation evaluation over these facts; that is M2."
verify = [{run = "cargo test --locked -p warrant-model declared", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- model", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.4"
title = "Spike S6: parity qualification against tsc --traceResolution"
delivers = "Creates warrant instrument qualify lang-ts per spec 6.8: the Node sidecar running the pinned typescript@7.0.2 with --traceResolution per owning tsconfig, the versioned trace adapter that binds each trace block to exactly one enumerated edge or fails, the per-edge comparison keyed by project, importer, span, specifier, and mode, the feature-and-mode coverage matrix, the clean-cache repeat, and the report; instrument lock parsing and warrant instrument status in warrant-evidence, which creates crates/core/src/schema/instruments.rs and schemas/warrant.instruments.json replacing their stubs and crates/cli/src/commands/instrument.rs as its command file and creates warrant/instruments.lock as the first lock; the capability report set to parity-qualified for exactly the covered modes; docs/spikes/2026-09-S6-parity.md; the decision it settles is which resolution modes lang-ts may call qualified"
kind = "spike"
blocked_by = ["W1.3", "W0.8"]
role = "executor"
persona = "repro-first-writer"
skills = ["rust-engineer", "research"]
owned_files = ["crates/lang-ts/src/parity", "crates/lang-ts/sidecar", "crates/evidence", "crates/cli/src/commands/instrument.rs", "warrant/instruments.lock", "docs/spikes/2026-09-S6-parity.md", "crates/core/src/schema/instruments.rs", "schemas/warrant.instruments.json"]
invariants = ["A single disagreement leaves that project unqualified; no repository-wide percentage is reported as a pass", "A trace block that cannot be bound to exactly one enumerated edge is a failure, never skipped"]
acceptance = "The report contains the matrix over the corpus with zero disagreements for every mode marked qualified, the two known divergences as its first rows with their outcomes, and identical canonical edge sets across two clean runs; warrant instrument status compares the lock to the installed typescript; cargo test --locked -p warrant-lang-ts parity passes with a planted disagreement case that leaves the project unqualified. Green does not prove parity on projects or features outside the matrix."
verify = [{run = "cargo test --locked -p warrant-lang-ts parity", expect = "exit 0"}, {run = "cargo test --locked -p warrant-evidence", expect = "exit 0"}, {run = "sh -c 'rg -q \"^Status: (met|not met)\" docs/spikes/2026-09-S6-parity.md'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs", "network"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W1.5"
title = "Spike S1: compiler-authority references on TypeScript 7"
delivers = "spikes/s1 with the three candidates from spec 6.6 (a Node indexer over typescript@7.0.2's unstable/async client emitting a Warrant-owned canonical index, the native language server over LSP, and the published scip-typescript artifact recorded as non-authoritative), measured on the corpus against one hundred hand-verified consumer lists; docs/spikes/2026-09-S1-compiler-references.md with precision, recall, determinism across two clean runs, request count, bytes, peak memory, unsupported constructs, network use, and pinned identities; a drafted D10 decision for Trey with the fallback stated"
kind = "spike"
blocked_by = ["W1.3", "W0.8"]
role = "executor"
persona = "repro-first-writer"
skills = ["research", "rust-engineer"]
owned_files = ["spikes/s1", "docs/spikes/2026-09-S1-compiler-references.md"]
consumes = ["crates/lang-ts", "tests/corpus/manifest.yaml", "scripts/corpus.sh"]
invariants = ["A TypeScript 5 or 6 result is never reported as TypeScript 7 authority"]
acceptance = "The report states each candidate's result against every exit criterion, the hand-verified sample and its labeling method, and met or not met overall; the D10 draft names the recommended surface or the binding-level fallback and the contracts that stay labeled. Green does not adopt anything; D10 is Trey's decision."
verify = [{run = "sh -c 'rg -q \"^Status: (met|not met)\" docs/spikes/2026-09-S1-compiler-references.md'", expect = "exit 0"}, {run = "sh -c 'rg -q \"D10\" docs/spikes/2026-09-S1-compiler-references.md'", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs", "network"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W1.D10"
title = "Decide D10: the compiler-reference surface"
delivers = "Trey's ruling on S1: adopt the recommended surface as a pinned instrument, or keep binding-level with the affected contracts labeled"
kind = "checkpoint"
blocked_by = ["W1.5"]
acceptance = "Spec section 20 records D10 with Trey's choice and the S1 report digest; the choice names which capability the lang-ts report may claim."
assignee = "trey"
gate = "judgment"
because = "the spec reserves D10 for Trey and the typed-claim mapping in M2 depends on which capability exists"
```

```toml task
id = "W1.16"
title = "Apply the D10 decision to lang-ts"
delivers = "Creates crates/lang-ts/src/references and docs/spikes/2026-09-S1-adoption.md: on adoption, the selected reference producer integrated as a pinned instrument in warrant/instruments.lock with its index read into symbol_consumers and the capability report claiming symbol_level compiler only for the qualified scope; on fallback, binding-level retained, the contracts that need more labeled enforcement-unsupported in new fixtures under tests/conformance/policy/labeled, and the report saying so; in both cases the decision alone changes no capability label, only the tested integration does"
kind = "build"
blocked_by = ["W1.D10", "W1.4"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/lang-ts/src/references", "warrant/instruments.lock", "docs/spikes/2026-09-S1-adoption.md", "tests/conformance/policy/labeled"]
consumes = ["spikes/s1", "docs/spikes/2026-09-S1-compiler-references.md", "crates/lang-ts"]
invariants = ["The capability report claims compiler-level references only when a pinned instrument produced them in tests; a D10 ruling without the integration leaves the label at binding"]
acceptance = "cargo test --locked -p warrant-lang-ts references passes with the branch D10 chose: either an index fixture populating consumers with basis compiler and the lock recording the instrument, or a fallback fixture asserting binding-level and the labeled contracts; the adoption report states which branch and cites D10. Green does not prove precision on symbols outside the S1 sample."
verify = [{run = "cargo test --locked -p warrant-lang-ts references", expect = "exit 0"}, {run = "sh -c 'rg -q \"^Branch: (adopted|fallback)\" docs/spikes/2026-09-S1-adoption.md'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.9"
title = "Implement the query surface"
delivers = "warrant query owner, interface, consumers, why, blast, similar (exact-name and structural only), unresolved, --sql with the read-only authorizer and caps, and --schema; warrant policy effective <path|symbol|module> printing every applying contract with its selector, authority, class, and governing entry; every row labeled by basis; modules populated from module contracts and ownership; why answered from the effective policy's governing allow or deny entry"
kind = "build"
blocked_by = ["W1.8", "W1.6"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "rust-agent-cli", "tdd"]
owned_files = ["crates/cli/src/commands/query.rs", "crates/cli/src/commands/policy.rs", "crates/model/src/query", "tests/conformance/context/query"]
consumes = ["crates/model", "crates/model/src/declared", "crates/lang-rust", "crates/core/src/policy"]
invariants = ["Every returned fact carries a basis of declared, observed-static, observed-test, or inferred, and similar never returns inferred without a judgment backend"]
acceptance = "Conformance cases for each question over a fixture with declared and observed interfaces pass, including a why that cites the contract, a blast that marks entrypoints and external consumers, and policy effective on a path with two applying contracts; --sql rejects a write and truncates with a hint at the row cap. Green does not prove retrieval under ordinary language; that is context and S7."
verify = [{run = "cargo test --locked -p warrant-cli --test conformance -- context", expect = "exit 0"}, {run = "cargo test --locked -p warrant-model", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.10"
title = "Implement census observe and propose"
delivers = "warrant-census: observe without a manifest or policy, discovering integrations from language and workspace manifests and reporting ambiguous units; units, observed imports and entrypoints, cohesion, cycles, and possible interfaces; propose writing a starter manifest and draft policy to new caller-selected paths with basis, uncertainty, and authority draft on every item, the deterministic grouping method recorded, and refusal to overwrite; warrant census observe and propose; the warrant.census schema replacing its stub"
kind = "build"
blocked_by = ["W1.9"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/census", "crates/cli/src/commands/census.rs", "tests/conformance/census", "crates/core/src/schema/census.rs", "schemas/warrant.census.json"]
invariants = ["Propose never writes to an existing path and never marks any proposed contract with an authority other than draft"]
acceptance = "Census conformance cases pass: observe on a fixture without warrant/, propose to a new directory producing a manifest and policy that warrant policy compile accepts, a rerun refusing the existing destination, and every item carrying its basis; propose on the Atlas corpus completes and its output is saved to WARRANT_CORPUS_DIR for W1.C1. Green does not prove the grouping matches intended architecture; a human decides that."
verify = [{run = "cargo test --locked -p warrant-census", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- census", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.11"
title = "Implement context and propose"
delivers = "Creates warrant context accepting a description, paths, or symbols: offline retrieval in the spec 12.3 order (exact symbols and paths, declared responsibility names and aliases, intent text, graph neighbors and examples), matched, ambiguous, no-match, and incomplete statuses, each match carrying owner, interface, examples, consumers, and required evidence with their bases, model_status current, stale, or unavailable from the model-inputs digest, --budget bounding the complete encoded output with omissions explicit and an error when too small for the status document; warrant propose with --description, --paths, and --patch applied exactly to its named base in a temporary snapshot, reuse candidates labeled exact-name, declared-responsibility, or structural-similarity; the context and propose schemas replacing their stubs; no persistence of prompt text"
kind = "build"
blocked_by = ["W1.9"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "rust-agent-cli", "tdd"]
owned_files = ["crates/core/src/retrieval", "crates/cli/src/commands/context.rs", "crates/cli/src/commands/propose.rs", "tests/conformance/context/retrieval", "tests/conformance/bounded", "crates/core/src/schema/context.rs", "crates/core/src/schema/propose.rs", "schemas/warrant.context.json", "schemas/warrant.propose.json"]
invariants = ["A no-match result never claims that no owner exists, and an unrelated prompt returns no-match rather than irrelevant candidates", "A capped result never cuts a JSON value and always lists the omitted scope"]
acceptance = "Context conformance cases pass for a match returning owner, example, consumers, and required evidence rows with bases, for vocabulary mismatch with a declared alias, multiple plausible owners (ambiguous with the missing discriminator), genuine novelty, an unrelated prompt, a stale model, and no model; bounded cases pass for a budget that forces omission and a budget below the status minimum; a propose --patch with mismatched context is rejected. Green does not qualify retrieval; S7 does."
verify = [{run = "cargo test --locked -p warrant-cli --test conformance -- context", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- bounded", expect = "exit 0"}, {run = "cargo test --locked -p warrant-core retrieval", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.12"
title = "Implement the map renderer"
delivers = "warrant-render and warrant map: Mermaid and JSON views of the model with modules, interfaces, entrypoints, stores, effects, and external consumers, scoped to a module and its neighbors, observed, declared, inferred, forbidden, and unresolved relationships distinguishable, unread files and unsupported analysis visible, source and policy identities in the output, and --diff between two snapshots; the warrant.map schema replacing its stub"
kind = "build"
blocked_by = ["W1.9"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/render", "crates/cli/src/commands/map.rs", "tests/conformance/adapters/map", "crates/core/src/schema/map.rs", "schemas/warrant.map.json"]
invariants = ["A rendered map carries the denominator and unread count of the model it renders and never omits an unresolved edge silently"]
acceptance = "cargo test --locked -p warrant-render passes; the map conformance case renders a fixture with one unresolved edge and one unread file and both appear in Mermaid and JSON; --diff lists added and removed relationships between two snapshots. Green does not prove diagram completeness beyond the model's."
verify = [{run = "cargo test --locked -p warrant-render", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- adapters", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.13"
title = "Build the Claude Code delivery adapter and record a harness run"
delivers = "warrant hook claude-code emitting session-start and prompt-submit hook configuration that runs a bounded map summary and a bounded warrant context result with a subprocess deadline shorter than the harness timeout; --install writing harness configuration only on explicit request; delivery status recorded without prompt logging; the warrant.hook schema replacing its stub; a recorded non-interactive Claude Code run in a disposable project showing a sentinel from the context result in model input before the model's first tool decision, plus recorded runs for the timeout case and the skipped or distrusted hook case; docs/spikes/2026-09-delivery-claude-code.md naming the harness version, events, tool families covered, and each run's evidence"
kind = "build"
blocked_by = ["W1.11", "W1.12"]
role = "executor"
persona = "repro-first-writer"
skills = ["cc-hooks", "rust-agent-cli", "tdd"]
owned_files = ["crates/cli/src/commands/hook.rs", "crates/cli/src/hooks", "tests/conformance/adapters/hook", "docs/spikes/2026-09-delivery-claude-code.md", "crates/core/src/schema/hook.rs", "schemas/warrant.hook.json"]
consumes = ["crates/core/src/retrieval", "crates/cli/src/commands/context.rs", "crates/render"]
invariants = ["The adapter's deadline is shorter than the harness timeout and a timeout returns a small failure status, never a partial context presented as complete", "A skipped, killed, or distrusted hook is recorded as unavailable, never as delivery"]
acceptance = "The record shows the sentinel in the transcript before the first tool call in the delivery run, the failure status in the timeout run, and unavailable in the skipped-hook run, with the Claude Code version and the exact events; the hook conformance area passes for the deadline and the no-prompt-logging cases; --install without the flag changes nothing. Green does not prove delivery on any other harness or on a cold model rebuild."
verify = [{run = "cargo test --locked -p warrant-cli --test conformance -- adapters", expect = "exit 0"}, {run = "sh -c 'rg -q \"^Status: (delivered|unavailable)\" docs/spikes/2026-09-delivery-claude-code.md'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs", "network"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W1.C1"
title = "Review the Atlas module and ownership declarations"
delivers = "Human-reviewed Atlas module, ownership, and alias declarations at the pinned commit, saved to WARRANT_CORPUS_DIR with their digest recorded in the corpus manifest"
kind = "checkpoint"
blocked_by = ["W1.10"]
acceptance = "The corpus manifest carries the digest of a declarations file Trey has read and amended, and the pin record says which module names are intended architecture rather than the census's observation."
assignee = "trey"
gate = "completion"
because = "S7 queries run against human-reviewed declarations, and only Trey knows Atlas's intended owners"
discharged_by = "the reviewed declarations file digest in tests/corpus/manifest.yaml and Trey's note in the pin record"
```

```toml task
id = "W1.14"
title = "Draft the S7 sample, failure-cost criteria, and output budget"
delivers = "docs/spikes/2026-09-S7-protocol.md: the retrieval qualification protocol from spec 12.3 and 17.7 with a proposed redacted request sample (grouped undo, reuse under different terminology, ambiguous ownership, legitimate novelty, vocabulary mismatch, unrelated prompts), labels per case, predeclared acceptance criteria with failure costs, the output budget, the development versus held-out split, and the delivery-timing measurements to record; the sample itself in WARRANT_CORPUS_DIR with its digest in the protocol"
kind = "build"
blocked_by = ["W1.C1"]
role = "executor"
persona = "repro-first-writer"
skills = ["write-human", "research"]
owned_files = ["docs/spikes/2026-09-S7-protocol.md"]
invariants = ["No Atlas transcript or private request text appears in the committed protocol; only digests and redacted labels"]
acceptance = "The protocol states every criterion as a number with a denominator, separates development from held-out queries, and names what would count as a failing result; a privacy scan over the file finds no Atlas source or transcript text. Green does not qualify anything; Trey approves the protocol at W1.C2."
verify = [{run = "sh -c 'test -f docs/spikes/2026-09-S7-protocol.md && rg -q \"held-out\" docs/spikes/2026-09-S7-protocol.md'", expect = "exit 0"}, {run = "sh -c 'rg -n \"/home/|\\.ts\\.net\" docs/spikes/2026-09-S7-protocol.md; test $? -eq 1'", expect = "exit 0"}]
reversibility = "reversible"
effects = ["docs"]

[review]
personas = ["spec-fidelity"]
```

```toml task
id = "W1.C2"
title = "Approve the S7 sample, criteria, and budget"
delivers = "Trey's approval of the frozen labeled sample, failure-cost criteria, and output budget, recorded under spec section 20"
kind = "checkpoint"
blocked_by = ["W1.14"]
acceptance = "Spec section 20 records the approval with the protocol's digest, or names the line to change; Trey confirms the committed protocol carries no private Atlas text, which no scan can establish."
assignee = "trey"
gate = "judgment"
because = "the spec requires Trey to approve and freeze the sample and criteria before held-out measurement, so a later result cannot be relabeled a pass"
```

```toml task
id = "W1.15"
title = "Spike S7: qualify retrieval and delivery"
delivers = "The S7 measurement under the approved protocol: found, missed, misleading, ambiguous, and no-match outcomes with denominators over the held-out queries, returned bytes and omissions, warm and cold latency, and delivery timing and failure behavior for the Claude Code adapter; docs/spikes/2026-09-S7-report.md stating met or not met per criterion; the retrieval_qualification record in warrant/instruments.lock binding the exact build digest, retrieval configuration, sample, criteria, and reference policy, set only if every criterion is met; tests/acceptance.d/04-context-atlas.sh and 05-delivery-record.sh added to REQUIRED; the decision it settles is whether retrieval and delivery are qualified for M1 exit"
kind = "spike"
blocked_by = ["W1.C2", "W1.13", "W1.4", "W1.16"]
role = "executor"
persona = "repro-first-writer"
skills = ["research", "rust-agent-cli"]
owned_files = ["docs/spikes/2026-09-S7-report.md", "warrant/instruments.lock", "tests/acceptance.d/04-context-atlas.sh", "tests/acceptance.d/05-delivery-record.sh", "tests/acceptance.d/REQUIRED"]
consumes = ["docs/spikes/2026-09-S7-protocol.md", "crates/cli/src/hooks", "crates/lang-ts/src/parity"]
invariants = ["retrieval.qualified is true only when the predeclared criteria were met on the held-out set; a report that exists but misses a criterion leaves it false"]
acceptance = "The report carries every criterion with its measured value and denominator, the protocol and sample digests, and the build and configuration digests; the lock's retrieval_qualification matches the report's result; the two acceptance.d items print ACCEPT. Green does not prove correctness on queries outside the sample."
verify = [{run = "sh -c 'rg -q \"^Status: (met|not met)\" docs/spikes/2026-09-S7-report.md'", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 5"}]
reversibility = "reversible"
effects = ["docs", "code", "network"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W1.G"
title = "Close M1 after the milestone review"
delivers = "A critical-regime review of the M1 diff at its HEAD with dispositions in docs/plans/reviews/, adopted majors fixed, and M1's exit criteria from spec section 21 checked: ordinary-language queries return real references under S7's criteria, the six retrieval cases are distinct, a recorded harness run proves before-decision delivery including nondelivery, qualified resolution has zero disagreements on covered features, Cargo exercises the language boundary"
kind = "checkpoint"
blocked_by = ["W1.15", "W1.12", "W1.4", "W1.16"]
acceptance = "docs/plans/reviews/<date>-M1.md exists with three reviewers from three families and the exit statement; S7 is met on the held-out set, retrieval.qualified is true in the lock, and the lock's build digest equals the reviewed HEAD's, so a review fix that changes the build or retrieval configuration sends S7 back to W1.15 before M1 closes; S1 is resolved through D10 and W1.16; a not-met S7 does not close M1 and goes to Trey as a retrieval rethink with the report attached."
assignee = "fable"
gate = "judgment"
because = "the spec makes every milestone diff a critical review, requires S1 resolved and S7 passed for M1 exit, and M2's evaluation builds on the model, policy, and retrieval M1 delivered"
```

```toml task
id = "W2.1"
title = "Implement obligation derivation and evaluation"
delivers = "Creates crates/core/src/eval: obligations from spec 8.1 for dependency (edge-forbidden, edge-not-allowed, cycle-forbidden), interface (interface-bypass, consumer-not-permitted), module ownership, inventory-owned, inventory-readable, resolution-complete, and declared-consistency; each obligation recording typed claim, required capabilities, actual basis, and limits; outcomes satisfied, unsatisfied, undeterminable with undeterminable never decaying; the obligations digest; snapshot-dependent policy lint (empty-selector unless may_be_empty, module overlap on actual files, drift findings); the claim-to-capability check against the integration's report so an unsupported requirement is enforcement-unsupported; the warrant.obligations schema replacing its stub"
kind = "build"
blocked_by = ["W1.G"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core/src/eval", "crates/cli/src/commands/policy.rs", "tests/conformance/claims", "crates/core/src/schema/obligations.rs", "schemas/warrant.obligations.json"]
invariants = ["An undeterminable outcome never resolves to satisfied under any exception, default, or flag", "An obligation's recorded basis is never stronger than the evidence that produced it"]
acceptance = "cargo test --locked -p warrant-core eval passes; the claims conformance area passes with cases for each obligation type, an unread file in scope producing undeterminable, a required capability the report lacks producing enforcement-unsupported, an empty selector failing lint, a required file made unreadable setting analysis incomplete with unread-in-scope (the section 5.7 proof assigned here), and drift reported without blocking; the undeterminable test was observed failing with the decay guard removed. Green does not prove pattern, evidence, or capability-link obligations."
verify = [{run = "cargo test --locked -p warrant-core eval", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- claims", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.7"
title = "Implement rulings, signing glue, verification, and exceptions"
delivers = "warrant-authority: the ruling Statement for amendment and exception kinds with distinct predicate types and sshsig namespaces under a configurable domain placeholder pending D2; RFC 8785 canonical bytes with the RFC's test vectors; drafted_by, signer, signed_at set by the signing tool; warrant rule draft for both kinds over supplied items (batch drafting from a receipt is W2.9), render with its deterministic rendered_digest, sign that renders the record's effects and classification from its final canonical bytes, asks the human to confirm, writes nothing on refusal, and then invokes ssh-keygen -Y sign with the kind's namespace and a human-supplied key path only, in a process that loads no candidate code, policy hook, or plugin, verify in-process with ssh-key checking principal, namespace, validity window, revocation, expiry, policy digest, candidate scope, and supersession, list, and stats; trust root and revoked-keys parsing; the exception lifecycle with dispositions, occurrence digests supplied as fixtures, applies_to_count, expiry on the gate clock; the RulingVerifier trait implementation for the core receipt verifier; a stub batch module pre-declared for W2.9 and called from rule draft, returning an empty batch; the warrant.ruling schema replacing its stub; the S4 fixture suite signed by ssh-keygen across ed25519, ecdsa, and rsa with tampered variants that both verifiers reject; docs/spikes/2026-09-S4-verification-oracle.md"
kind = "build"
blocked_by = ["W1.G"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd", "house-bar"]
owned_files = ["crates/authority", "crates/cli/src/commands/rule.rs", "tests/conformance/authority", "docs/spikes/2026-09-S4-verification-oracle.md", "crates/core/src/schema/ruling.rs", "schemas/warrant.ruling.json"]
invariants = ["Warrant never performs a private-key operation and never reads a key from the repository; sign only shells to ssh-keygen with a caller-supplied key path, after rendering the final bytes and receiving confirmation", "No durable signed record is produced under the placeholder domain outside test fixtures; the fixture keys are synthetic and named as such (D2 stays open)", "Any byte change to a signed record invalidates it; verification compares bytes and never re-serializes", "An exception-only key cannot produce a valid policy amendment"]
acceptance = "cargo test --locked -p warrant-authority passes; the authority conformance area passes with the S4 matrix (three key types, valid and each tampered input: bytes, namespace, principal, validity window, revocation) where in-process and ssh-keygen -Y verify agree on every case, an expired exception satisfying nothing, a batch with an added item failing, a draft that cannot be signed while an item lacks a reason, a sign run whose confirmation is declined writing no signature, and a sign run whose rendered digest differs from the record's canonical bytes refusing; rule sign with no --key exits 2. Green does not prove D2's final identifiers or a protected trust root deployment."
verify = [{run = "cargo test --locked -p warrant-authority", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- authority", expect = "exit 0"}, {run = "sh -c 'rg -q \"^Status: (met|not met)\" docs/spikes/2026-09-S4-verification-oracle.md'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.12"
title = "Draft the early comparison protocol and rubric"
delivers = "Creates docs/acceptance/2026-M2-comparison-protocol.md: the spec 17.5 early advisory comparison as a runnable protocol using existing agent tools: frozen Atlas source and reference policy, checker and instrument versions, model and harness settings, the request set (grouped undo, reuse under different terminology, ambiguous ownership, legitimate new responsibility, combined-source invalidation), the two arms, the review rubric for the six outcomes with raw-count definitions and denominators, the human-decision budget, blinding where feasible, what result would count as no benefit, mixed, or benefit, and a frozen-inputs section that points at docs/acceptance/2026-M2-frozen-inputs.md, which W2.11 fills"
kind = "build"
blocked_by = ["W1.G"]
role = "executor"
persona = "repro-first-writer"
skills = ["write-human", "research"]
owned_files = ["docs/acceptance/2026-M2-comparison-protocol.md"]
invariants = ["Every outcome is a raw count with a stated denominator; the protocol defines no composite score"]
acceptance = "The protocol names both arms, every frozen input by digest or version, the six outcome measurements with guards from spec 17.5, and the three result classes; it contains no time estimate. Green does not approve the rubric; W2.C1 does."
verify = [{run = "sh -c 'rg -q \"no benefit\" docs/acceptance/2026-M2-comparison-protocol.md && rg -q \"denominator\" docs/acceptance/2026-M2-comparison-protocol.md'", expect = "exit 0"}]
reversibility = "reversible"
effects = ["docs"]

[review]
personas = ["spec-fidelity"]
```

```toml task
id = "W2.2"
title = "Implement pattern contracts, registries, and structural capability links"
delivers = "crates/core/src/pattern over ast-grep-core with tree-sitter-typescript: pattern-forbidden and pattern-required obligations within a scope; registry declarations, matched through W1.8's registry primitive, producing capability instances and the section 5.7 proof that a removed registry entrypoint yields a finding on the capability that relied on it; effect required_context and caller-not-permitted, state recognized write sites and writer-not-permitted, capability structural links (entrypoint, authorization-call-occurrence, compensation-owner) as link-missing; data sink-not-permitted at binding level; every structural result labeled with its pattern basis and limits"
kind = "build"
blocked_by = ["W2.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core/src/pattern", "tests/conformance/claims/pattern"]
consumes = ["crates/core/src/eval", "crates/core/src/pattern/registry.rs"]
invariants = ["A call occurrence never satisfies a behavior claim; the obligation records pattern basis and its limits verbatim from the contract", "A registry pattern that matches nothing yields a drift finding on the registry, not silent absence of instances"]
acceptance = "Pattern conformance cases pass for each contract kind's structural obligation, for a registry with no matches, for a declared registry entrypoint removed from source (finding on the dependent capability), for an authorization call that occurs but does not guard (structure present, behavior link still required), and for a recognized ORM write rule that omits another write form (the narrower claim named). Green does not prove behavior; evidence links are W2.4."
verify = [{run = "cargo test --locked -p warrant-core pattern", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- claims", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W2.4"
title = "Implement evidence binding, the attest wrapper, and two report adapters"
delivers = "warrant-evidence extended: evidence kinds; warrant attest run recording an advisory test-receipt with snapshot, command, digests, recorded env allowlist, report files, instruments, and provenance agent-local-advisory; the execution contract fields present and unset; limits recorded as not-applied with reason where RLIMIT is not measured (S5 carried forward); warrant evidence import --instrument --receipt and --attestation with report-unbound for neither; vitest-json and fallow-json adapters as versioned instruments with fixture sets, Fallow's exit 1 read by its declared exit contract, an empty or unparseable report as report-truncated; evidence-required obligations in crates/core/src/evidence evaluated to satisfied, missing, or stale with class, unit, tag, and scenario matching, and report predicates evaluated over canonical evidence rows: absent-in-scope for a rule, and at_least or at_most on a derived metric under the adapter's pinned formula, so a bound report whose contents violate the requirement is unsatisfied; warrant evidence list over the stored receipts and imports; the test-receipt and evidence schemas replacing their stubs"
kind = "build"
blocked_by = ["W2.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/evidence", "crates/core/src/evidence", "crates/cli/src/commands/attest.rs", "crates/cli/src/commands/evidence.rs", "tests/conformance/execution", "crates/core/src/schema/test-receipt.rs", "crates/core/src/schema/evidence.rs", "schemas/warrant.test-receipt.json", "schemas/warrant.evidence.json"]
consumes = ["crates/core/src/eval", "crates/core/src/policy"]
invariants = ["An instrument's finding set comes from its parsed report, never from its exit status; exit zero with an empty report is report-truncated", "A receipt for a different tree is stale and satisfies nothing; an unbound report satisfies nothing", "A bound report satisfies an evidence requirement only when its predicate holds over the canonical rows; presence of the report is never sufficient"]
acceptance = "Execution conformance cases pass for a receipt on the exact tree (satisfied), a different tree (stale), an unbound report (report-unbound), Fallow exit 1 with findings (findings read), zero-case and truncated vitest reports (unsatisfied), a class label without a service binding (recorded but not credited), a bound Fallow report containing a forbidden finding in scope (unsatisfied), a metric below its threshold (unsatisfied) and above it (satisfied); adapter fixtures pin both formats. Green does not prove protected-producer admission; every receipt here is advisory."
verify = [{run = "cargo test --locked -p warrant-evidence", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- execution", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W2.6"
title = "Implement signals and the taste profile"
delivers = "crates/core/src/signals: metric-gaming and conceptual-expansion signals from spec 8.5 computed between a candidate and a supplied base with counts and locations and no score, each with its counterexample text from the profile; crates/core/src/profile: the warrant.profile schema with measurable contracts and evidence, signal consequences, and judgment questions parsed and validated, judgment backend none accepted and advice recorded as missing when absent; preferences with on_violation note producing counted advice, never obligations; the warrant.profile schema replacing its stub"
kind = "build"
blocked_by = ["W2.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core/src/signals", "crates/core/src/profile", "tests/conformance/verdict/signals", "crates/core/src/schema/profile.rs", "schemas/warrant.profile.json"]
consumes = ["crates/core/src/eval", "crates/core/src/policy"]
invariants = ["No signal or profile output is a scalar composite; every number is a count of a named thing", "A required comparison without its base is incomplete, never zero change"]
acceptance = "Signal conformance cases pass for suppressions added, tests removed from a required unit, a new store owner, a new cycle, and change locality, each with count and locations; a profile that promotes a preference to fail compiles only as a classified policy change; a missing base yields incomplete. Green does not prove intent to game; the spec says so."
verify = [{run = "cargo test --locked -p warrant-core signals", expect = "exit 0"}, {run = "cargo test --locked -p warrant-core profile", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- verdict", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W2.8"
title = "Implement the policy-widening classifier and amendment chain"
delivers = "crates/core/src/classify: comparison of canonical rules across the dimensions of spec 11.5 with a partial order per dimension, classes narrowing, widening, restructuring, and uncertain, proof rules limited to the supported transformations (identical rules after movement, literal set inclusion on unchanged scopes, independent requirement addition or removal, monotone thresholds), uncertain elsewhere and for mixed changes; warrant policy diff --base --candidate showing rule-level class, proof rule, and separate snapshot impact; amendment payloads policy-change, instrument-adoption, and accepted-limitation with before and after digests; the approved-policy anchor record and chain verification from anchor to proposed replacement; the warrant.diff schema replacing its stub"
kind = "build"
blocked_by = ["W2.7", "W2.1"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd", "house-bar"]
owned_files = ["crates/core/src/classify", "crates/cli/src/commands/policy.rs", "tests/conformance/identity", "crates/core/src/schema/diff.rs", "schemas/warrant.diff.json"]
consumes = ["crates/authority", "crates/core/src/policy", "crates/core/src/eval"]
invariants = ["Uncertain receives widening treatment everywhere the class is consumed", "Equal obligation sets on one snapshot never establish restructuring; a glob replaced by today's file list is uncertain"]
acceptance = "Identity conformance cases pass for removed contract (widening), added invariant (narrowing), file move and reorder (restructuring), glob to file list (uncertain), lowered threshold (widening) and raised (narrowing), instrument version change (uncertain), mixed change (uncertain with parts preserved), and a candidate-supplied base that cannot replace the anchor; the amendment chain rejects a missing superseded record. Green does not prove equivalence for rule grammar outside the proven transformations."
verify = [{run = "cargo test --locked -p warrant-core classify", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- identity", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.3"
title = "Implement findings, explanations, grouping, and stable identity"
delivers = "Creates crates/core/src/findings: the warrant.finding document with the seven explanation fields (null with null_reason when absent), consequence from on_violation, remediation with code and amendment alternatives, the focused check string; grouping by computed cause with primary and symptoms; finding.id as sixteen base32 characters of sha256 over contract, obligation, structural subject, and basis kind with no positions; rename mapping through git rename detection with renamed_from uncertain when unattributed; occurrence-evidence digests per evaluator; warrant explain <finding | cause> [--show-source] as a bounded read; the warrant.finding schema replacing its stub"
kind = "build"
blocked_by = ["W2.2"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd"]
owned_files = ["crates/core/src/findings", "crates/cli/src/commands/explain.rs", "tests/conformance/identity/findings", "crates/core/src/schema/finding.rs", "schemas/warrant.finding.json"]
invariants = ["A finding's id never changes when only its line or column moves, and never survives a change of subject", "One wrong registration produces one primary finding with symptom counts, not one finding per consumer"]
acceptance = "Findings conformance cases pass for a moved line (same id), a renamed file with confident detection (mapped) and without (renamed_from uncertain), a replaced occurrence with the same count (new occurrence digest), an unrelated nearby edit (same digest), and a registry mismatch grouping every link-missing under one cause; explain prints all seven fields for a fixture finding. Green does not prove exception applicability; W2.7 consumes these digests."
verify = [{run = "cargo test --locked -p warrant-core findings", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- identity", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code"]

[review]
personas = ["refuter", "conventions", "test-skeptic"]
```

```toml task
id = "W2.5"
title = "Implement the four-facet verdict, check, and human rendering"
delivers = "crates/core/src/verdict: compliance, analysis, evidence, and approval computed independently; acceptance derived; the warrant.verdict document with denominator, findings, signals, judgments empty, tool identity; the check receipt statement in crates/core/src/receipt (in-toto Statement, distinct check predicate type, RFC 8785 canonical bytes with no floats, receipt id as sha256 of those bytes, subject carrying tree id and inventory digest separately, uncaptured_inputs) emitted by every check so no verdict exists without its receipt; approval computed from verified exceptions and from the widening classifier's items; mode check with check-clean and check-blocked, narrowed scope recorded with omitted obligations, mode diagnostic without acceptance; exit code mapping; warrant check with --snapshot, --changed, --index, --base, and --compare <receipt> under spec 13.5's comparability rules with introduced-relative-to-comparisons and undetermined, and --contract <id> --instance <subject> running exactly the focused check a finding names; the human paragraph rendering nothing the JSON lacks; tests/acceptance.d/06-stories-abc.sh added to REQUIRED; the verdict and receipt schemas replacing their stubs"
kind = "build"
blocked_by = ["W2.3", "W2.4", "W2.6", "W2.7", "W2.8"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "rust-agent-cli", "tdd"]
owned_files = ["crates/core/src/verdict", "crates/core/src/receipt", "crates/cli/src/commands/check.rs", "tests/conformance/verdict", "tests/conformance/combination", "tests/acceptance.d/06-stories-abc.sh", "crates/core/src/schema/verdict.rs", "crates/core/src/schema/receipt.rs", "schemas/warrant.verdict.json", "schemas/warrant.receipt.json", "tests/acceptance.d/REQUIRED"]
consumes = ["crates/core/src/findings", "crates/core/src/evidence", "crates/core/src/signals", "crates/authority", "crates/core/src/classify"]
invariants = ["No facet reads another facet's value; a run can be compliance fail and analysis incomplete at once and shows both", "A check result can never carry mode gate or acceptance accepted"]
acceptance = "Verdict conformance cases pass for stories A, B, and C, for a simultaneous fail and incomplete, for a facet-input matrix where each facet's inputs are mutated one at a time and only that facet changes, for a diagnostic with provisional parse errors and no acceptance field, for a focused check reproducing one finding, and for exit codes 0, 1, 2 on the matching fixtures; combination cases pass for two passing revisions whose combination fails, attributed relative to the supplied comparisons, and for missing comparison inputs yielding undetermined; the acceptance item prints ACCEPT 6. Green does not prove gate acceptance; there is no gate command yet."
verify = [{run = "cargo test --locked -p warrant-core verdict", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- verdict", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- combination", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 6"}]
reversibility = "reversible"
effects = ["code"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.9"
title = "Implement the receipt verifier and exception batches from receipts"
delivers = "crates/core/src/receipt/verify.rs and warrant verify performing spec 10.5 steps 1 to 9 in order through the injected RulingVerifier, the gate predicate shape defined but unissued, with each step's result, --offline and --recompute, results verified, verified-with-unchecked, stale, advisory, failed, and envelope-unsupported for a carrier other than sshsig-detached; the VSA mapping as a conformance fixture without the export command; warrant rule draft --kind exception --from <receipt> producing an itemized batch in crates/authority/src/batch.rs; the verify-under-one-second budget asserted in the corpus stage; the Recompute and Artifacts trait implementations composed in warrant-cli over snapshot, inventory, model, evidence, authority, and the integrations; the warrant.verify schema replacing its stub; items 07 to 09 added to REQUIRED; tests/acceptance.d/07-rule-verify-oracle.sh, 08-policy-diff.sh, 09-verify-reformatted.sh"
kind = "build"
blocked_by = ["W2.5", "W2.7", "W2.8"]
role = "executor"
persona = "minimalist-implementer"
skills = ["rust-engineer", "tdd", "house-bar"]
owned_files = ["crates/core/src/receipt/verify.rs", "crates/authority/src/batch.rs", "crates/cli/src/commands/verify.rs", "crates/cli/src/commands/rule.rs", "scripts/stages/45-corpus.sh", "tests/conformance/verdict/receipt", "tests/acceptance.d/07-rule-verify-oracle.sh", "tests/acceptance.d/08-policy-diff.sh", "tests/acceptance.d/09-verify-reformatted.sh", "crates/core/src/schema/verify.rs", "schemas/warrant.verify.json", "tests/acceptance.d/REQUIRED"]
consumes = ["crates/core/src/receipt", "crates/authority", "crates/core/src/classify", "crates/core/src/findings"]
invariants = ["A reformatted receipt fails at step 1; verification never re-serializes", "A verified-with-unchecked result is never reported as an acceptance credential", "An unsigned receipt verifies to advisory at best; verified requires a producer signature and recomputation"]
acceptance = "Receipt conformance cases pass for an unsigned check receipt (every step reported, result advisory), the same receipt signed with the synthetic producer key without --recompute (verified-with-unchecked) and with it (verified, still not an acceptance credential), a reformatted copy (failed at step 1), a --recompute case that traverses the real composition and fails when the fixture's inventory is altered, an altered lock at the subject tree (step 6 failed), a required artifact missing from the store (unchecked, not acceptance) and one present with a mismatched subject (failed), all through the real CLI composition, a missing evidence artifact (unchecked, not acceptance), an expired ruling relied on (stale), and the VSA mapping fixture; an exception batch drafted from a receipt matches its findings item for item; the three acceptance items print ACCEPT. Green does not prove a protected producer signature; none exists before M3."
verify = [{run = "cargo test --locked -p warrant-core receipt", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- verdict", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 9"}]
reversibility = "reversible"
effects = ["code"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.10"
title = "Add mutation testing, fault injection, and adversarial conformance"
delivers = "scripts/stages/60-mutants.sh running cargo-mutants over crates/core and crates/authority with any surviving mutant failing, other crates reporting only; fault-injection test builds exercising parse failure, missing instruments, report truncation, expired rulings, unstable source, invalid snapshot input, untrusted configuration, and output truncation through every output path; tests/conformance/adversarial with every row of spec 17.2's table that does not require the D5 deployment or S7's harness runs, plus the offline area (model service absent, acceptance facets unchanged) and the inline-approval-prose case; tests/acceptance.d/01-gate.sh added to REQUIRED and 60-mutants.sh added to scripts/stages/REQUIRED"
kind = "build"
blocked_by = ["W2.8", "W2.9"]
role = "executor"
persona = "repro-first-writer"
skills = ["rust-engineer", "tdd", "house-bar"]
owned_files = ["scripts/stages/60-mutants.sh", "crates/core/src/fault.rs", "tests/conformance/adversarial", "tests/conformance/offline", "tests/acceptance.d/01-gate.sh", "scripts/stages/REQUIRED", "tests/acceptance.d/REQUIRED"]
consumes = ["crates/core/src/classify", "crates/core/src/receipt", "crates/core/src/verdict", "crates/authority"]
invariants = ["Zero surviving mutants in core and authority at HEAD; a planted no-op mutation of the facet derivation is caught", "Missing authority or evidence cannot become acceptance in any adversarial case"]
acceptance = "scripts/gate.sh passes with the mutants stage; the adversarial and offline areas pass and tests/conformance/adversarial/README.md maps every implemented 17.2 row to its case and lists the rows deferred to M3 with the reason; the acceptance item prints ACCEPT 1. Green does not prove protected-runner isolation; those rows are deferred by name."
verify = [{run = "scripts/gate.sh", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- adversarial", expect = "exit 0"}, {run = "cargo test --locked -p warrant-cli --test conformance -- offline", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 1"}]
reversibility = "reversible"
effects = ["code"]

[review]
tier = "critical"
personas = ["refuter", "attacker", "test-skeptic"]
```

```toml task
id = "W2.11"
title = "Dogfood the advisory check on Atlas and record the onboarding burden"
delivers = "Creates docs/acceptance/2026-M2-atlas-dogfood.md and this repository's own warrant/warrant.yaml and warrant/policy: warrant check on the pinned Atlas corpus under the reviewed declarations and a drafted contract set in the section 7.8 shape (kept in WARRANT_CORPUS_DIR), recording the onboarding exception burden from rule draft --kind exception --from, re-sign requests caused by occurrence drift across two Atlas revisions, policy-change classifications from a drafted amendment, cold-run wall-clock and peak memory asserted in the corpus stage against spec 17.6's cold budget (under twenty seconds and one gibibyte, failing on regression), and every incomplete-analysis reason; creates docs/acceptance/2026-M2-frozen-inputs.md as a draft of the reference policy, checker build, instrument lock, and corpus identities, which W2.14 finalizes after review and requalification and which the protocol references by path; writes the budget assertions into scripts/stages/45-corpus.sh; Warrant's own workspace checked under warrant/policy/workspace.yaml with the Cargo integration replacing scripts/deps.sh in a rewritten scripts/stages/35-deps.sh; the decision it informs is R14, by putting the onboarding burden in front of Trey"
kind = "spike"
blocked_by = ["W2.10"]
role = "executor"
persona = "repro-first-writer"
skills = ["rust-agent-cli", "research", "write-human"]
owned_files = ["docs/acceptance/2026-M2-atlas-dogfood.md", "docs/acceptance/2026-M2-frozen-inputs.md", "warrant/warrant.yaml", "warrant/policy", "scripts/stages/35-deps.sh", "scripts/stages/45-corpus.sh"]
consumes = ["scripts/stages/60-mutants.sh", "crates/lang-rust", "crates/core/src/verdict", "tests/corpus/manifest.yaml"]
invariants = ["Every count in the dogfood record has its denominator and no Atlas source text is committed"]
acceptance = "The record carries the counts named in the delivers with denominators and the memory and wall-clock figures; warrant check on this repository under its own policy exits 0 with analysis complete for the Cargo edges; the deps stage passes through the Cargo integration and its self-test still detects a planted violation; the corpus stage fails when the cold budget is exceeded, shown by the lane running it once with the budget set to zero. Green does not prove Atlas's drafted contracts are its intended architecture."
verify = [{run = "sh -c 'cargo run -q --bin warrant -- check --snapshot worktree --format json | python3 -c \"import json,sys; d=json.load(sys.stdin); assert d[\\\"analysis\\\"][\\\"status\\\"]==\\\"complete\\\", d[\\\"analysis\\\"]\"'", expect = "exit 0"}, {run = "scripts/stages/35-deps.sh --self-test", expect = "self-test: violation detected"}, {run = "sh -c 'rg -n \"/home/|\\.ts\\.net\" docs/acceptance/2026-M2-atlas-dogfood.md; test $? -eq 1'", expect = "exit 0"}, {run = "scripts/gate.sh", expect = "exit 0"}]
reversibility = "reversible"
effects = ["code", "docs"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W2.R"
title = "Close M2 code after the milestone review"
delivers = "A critical-regime review of the complete M2 diff at its HEAD, after every M2 code write, with dispositions in docs/plans/reviews/ and adopted majors fixed, before the checker build is frozen for requalification and comparison"
kind = "checkpoint"
blocked_by = ["W2.11"]
acceptance = "docs/plans/reviews/<date>-M2.md exists with three reviewers from three families, dispositions for every finding, no open majors, and the HEAD it reviewed, which is the build W2.14 and W2.13 then use."
assignee = "fable"
gate = "judgment"
because = "the spec makes every milestone diff a critical review; the R14 product decision does not substitute for the code review"
```

```toml task
id = "W2.14"
title = "Requalify retrieval on the M2 checker build"
delivers = "Creates docs/spikes/2026-09-S7-requalification.md: the approved S7 protocol rerun with the same frozen sample, criteria, and budget, against the M2 build at the reviewed HEAD, its retrieval configuration, and the full Atlas reference policy W2.11 drafted for the comparison (a new policy identity, recorded, since the M1 run used the declarations only), with found, missed, misleading, ambiguous, and no-match counts and denominators, delivery timing for the Claude Code adapter, and writes warrant/instruments.lock with the retrieval_qualification record rebound to this build, configuration, and policy; creates tests/conformance/qualification with the matching-identities case and the mismatched-policy-digest case, run by the W0.6 conformance runner; finalizes docs/acceptance/2026-M2-frozen-inputs.md after the review fixes and the requalification so the checker build, lock, corpus, and reference policy identities it records are the ones the comparison will run; the decision it settles is whether the build the comparison will use is qualified"
kind = "spike"
blocked_by = ["W2.R"]
role = "executor"
persona = "repro-first-writer"
skills = ["research", "rust-agent-cli"]
owned_files = ["docs/spikes/2026-09-S7-requalification.md", "docs/acceptance/2026-M2-frozen-inputs.md", "warrant/instruments.lock", "tests/conformance/qualification"]
consumes = ["docs/spikes/2026-09-S7-protocol.md", "docs/spikes/2026-09-S7-report.md", "crates/cli/src/hooks"]
invariants = ["retrieval.qualified binds the exact build, retrieval configuration, sample, criteria, and reference policy; a changed build or policy without a met rerun leaves it false", "The frozen-inputs record, the lock record, and the comparison inputs name identical identities; a mismatch refuses qualification credit"]
acceptance = "The report states met or not met per criterion with the same denominators as W1.15, names the build digest, the protocol digest, and the reference policy digest, and the lock record and the frozen-inputs record carry the same identities; the qualification conformance area passes with the mismatched-policy case refused credit; a not-met result leaves qualified false and blocks W2.C1. Green does not prove correctness outside the sample."
verify = [{run = "sh -c 'rg -q \"^Status: (met|not met)\" docs/spikes/2026-09-S7-requalification.md'", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 4"}]
reversibility = "reversible"
effects = ["docs", "code", "network"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W2.C1"
title = "Approve the reference policy, comparison rubric, and usefulness criteria"
delivers = "Trey's approval of the full Atlas reference policy W2.11 drafted (effects, capabilities, evidence requirements, consequences, not only the W1.C1 declarations), the early comparison protocol together with the frozen-inputs record, the review rubric, and what reduction in unnecessary concepts and human intervention would count as useful"
kind = "checkpoint"
blocked_by = ["W2.12", "W2.14"]
acceptance = "Spec section 20 or the protocol's status line records Trey's approval with the protocol digest and the reference policy digest, and W2.14 reports met against that same policy; Trey's approval names the identities in the frozen-inputs record, which must equal the lock record and the executable, configuration, corpus, and policy W2.13 then uses; any later change to any of them voids the approval and returns here."
assignee = "trey"
gate = "judgment"
because = "spec 17.5 freezes the human-reviewed reference policy and rubric before agents run, so neither can be changed afterward to manufacture a pass"
```

```toml task
id = "W2.13"
title = "Run the early advisory comparison"
delivers = "The spec 17.5 early comparison executed under the approved protocol with existing delegate tooling: both arms on the frozen Atlas source and request set, the same fixed advisory checker and behavior tests, context delivery as the only intended difference; raw counts with denominators for reuse and simplicity, repair burden, human attention, correctness, retrieval usefulness, and the follow-on maintainability change; retries, unavailable delivery, and interventions visible; reviewer assessments blinded where feasible with disagreements shown; docs/acceptance/2026-M2-early-comparison.md classifying the result as benefit, mixed, or no benefit per the protocol; tests/acceptance.d/10-comparison-report.sh added to REQUIRED; the decision it feeds is R14"
kind = "spike"
blocked_by = ["W2.C1"]
role = "executor"
persona = "repro-first-writer"
skills = ["delegate-workflows", "research", "write-human"]
owned_files = ["docs/acceptance/2026-M2-early-comparison.md", "tests/acceptance.d/10-comparison-report.sh", "tests/acceptance.d/REQUIRED"]
consumes = ["docs/acceptance/2026-M2-comparison-protocol.md", "docs/spikes/2026-09-S7-requalification.md", "warrant/policy"]
invariants = ["The reference policy and result collection stay outside the implementing agents' control, and no rubric line changes after the runs begin"]
acceptance = "The report carries every outcome as raw counts with denominators for both arms, the protocol digest, per-request retries and interventions, reviewer disagreements, and the result class stated in the protocol's own terms; no Atlas source text is committed; the acceptance item prints ACCEPT 10. Green does not decide anything; R14 is Trey's."
verify = [{run = "sh -c 'rg -q \"^Result: (benefit|mixed|no benefit)\" docs/acceptance/2026-M2-early-comparison.md'", expect = "exit 0"}, {run = "sh -c 'rg -n \"/home/|\\.ts\\.net\" docs/acceptance/2026-M2-early-comparison.md; test $? -eq 1'", expect = "exit 0"}, {run = "tests/acceptance.sh", expect = "ACCEPT 10"}]
reversibility = "reversible"
effects = ["docs", "code", "network"]

[review]
personas = ["refuter", "provenance-auditor", "test-skeptic"]
```

```toml task
id = "W2.R2"
title = "Review the M2 delta after the comparison"
delivers = "A critical-regime review of everything written after W2.R's HEAD (the requalification report, the frozen-inputs record, the lock, the comparison report, acceptance item 10) with dispositions in docs/plans/reviews/, confirming the measured checker and frozen inputs did not change; if they did, the work returns to W2.14 and W2.C1 before R14"
kind = "checkpoint"
blocked_by = ["W2.13"]
acceptance = "docs/plans/reviews/<date>-M2-delta.md exists with three reviewers from three families, no open majors, and a statement that the comparison ran on the identities W2.C1 approved."
assignee = "fable"
gate = "judgment"
because = "the spec's milestone review must cover the complete M2 diff, and the comparison's own artifacts land after the code review"
```

```toml task
id = "W2.G"
title = "R14: proceed to M3, rethink the guidance, or rule on a mixed result"
delivers = "Trey's decision on the early comparison: authorize planning M3 to M5, send the guidance back for revision and a rerun, or rule on a mixed result"
kind = "checkpoint"
blocked_by = ["W2.R2"]
acceptance = "Spec section 20 records the R14 outcome with the comparison report digest and Trey's decision; docs/plans/live-state.md names the next leg."
assignee = "trey"
gate = "judgment"
because = "R14 is the spec's stop-and-rethink checkpoint and only Trey decides whether the protected acceptance system gets built"
```

## Invariant → verify table

Every invariant maps to a verify row and the evidence that discriminates a bound check from a decorative one.

| Task | Invariant | Verify row and discriminating evidence |
|---|---|---|
| W0.1 | scripts/deps.sh fails on a forbidden crate edge and its self-test proves the detector binds | `scripts/deps.sh --self-test` plants a `cli` dependency inside `core` in a temporary copy and must print the detection line; a detector that always passes prints nothing and the row fails |
| W0.1 | scripts/gate.sh fails when a stage named in REQUIRED is missing or not executable, and fails when any stage fails | `scripts/gate.sh` exit 0 at HEAD; the lane records one run with a REQUIRED stage renamed away and one with a stage exiting 1, both failing the gate |
| W0.2 | Every emitted document type carries schema_version and regenerates its checked-in schema byte-identically | `scripts/stages/40-schema.sh` regenerates into a temp dir and diffs; the lane records the run with one schema hand-edited failing |
| W0.2 | The manifest parser rejects an unknown field and an unknown class name with a typed error naming the field | `cargo test --locked -p warrant-core` includes the two rejection tests, observed red before the validation existed |
| W0.8 | No home-directory path, internal hostname, or credential appears in any committed corpus file | The privacy scan verify row over docs/corpus, tests/corpus, and scripts/corpus.sh must find nothing; the row fails when a match exists |
| W0.8 | scripts/corpus.sh verify fails when a tarball digest does not match the manifest | `scripts/corpus.sh verify` exit 0; the lane records the run after one byte of the tarball changed exiting 1 |
| W0.3 | An index or commit snapshot makes zero worktree read attempts | `cargo test --locked -p warrant-snapshot` dirties the worktree after staging, takes index and commit snapshots, and asserts the read-attempt counter at the single worktree read function is zero; a worktree snapshot in the same test drives it above zero, proving the counter binds |
| W0.3 | A worktree read whose blob no longer matches the captured id fails closed after one retake with snapshot-unstable and no artifact | The same suite rewrites a file between capture and read twice and asserts the error and an empty cache directory; observed red before the retake logic |
| W0.5 | A first-party file outside every module selector is listed under unowned_source, never silently classified | `cargo test --locked -p warrant-inventory` asserts the entry has `module: null` and appears in the summary list |
| W0.5 | Overlapping module selectors and conflicting explicit class assignments are lint errors, never resolved by order | Suite asserts both error kinds and that swapping declaration order changes nothing |
| W0.4 | Identical tree ids between native capture and git write-tree over every S3 case, or the feature stays off | `cargo test --locked -p warrant-snapshot --all-features` compares ids per case; the report's status line matches the feature default |
| W0.7 | Machine output never contains an ANSI escape or progress text | `cargo test --locked -p warrant-cli` pipes every command's stdout and parses the whole stream as JSON or NDJSON; any progress line or escape byte fails the parse |
| W0.7 | A cache artifact is readable only after its atomic rename; an interrupted write leaves no partial file | Suite sends SIGTERM mid-capture and asserts no non-temporary file exists and exit 143 |
| W0.6 | Every case's break-the-control variant fails the runner when the guarded control is disabled | Runner conformance runs; the lane records a scratch build with the unowned-source check disabled failing the inventory break case |
| W0.6 | The runner compares expect.json semantically and never byte-for-byte | Runner test reorders keys in an expect file and still passes |
| W1.1 | Equal semantic rows produce an equal model digest regardless of insertion order or timestamps | `cargo test --locked -p warrant-model` shuffles insertion and asserts equal digests |
| W1.1 | A query connection cannot execute INSERT, UPDATE, DELETE, ATTACH, or PRAGMA writes | Suite attempts each statement kind and asserts the authorizer error; observed red with the authorizer removed |
| W1.7 | Reordering contracts across files or reformatting YAML leaves the policy digest unchanged | Policy conformance digest-stability case compares digests across three layouts |
| W1.7 | A claim whose required capabilities a user lowers by editing a field is rejected at compile | Suite compiles a contract with an edited mapping and asserts the compile error |
| W1.2 | A non-literal dynamic import is recorded as unsupported with dynamic-nonliteral, never resolved by guess | Model conformance case asserts the unsupported row and no edge |
| W1.2 | The capability report never claims a construct the analyzer did not observe in tests | Suite asserts every `supports` entry has a fixture that produced at least one edge of that kind |
| W1.6 | The Cargo integration claims authority only for package edges and reports every other reference kind as unsupported | `cargo test --locked -p warrant-lang-rust` asserts the report's `symbol_level: none` and unsupported list |
| W1.3 | An unresolved edge is stored with its reason and counted in the analysis facet inputs, never dropped | Model conformance case with a missing export condition asserts the unresolved row and reason |
| W1.4 | A single disagreement leaves that project unqualified; no repository-wide percentage is reported as a pass | `cargo test --locked -p warrant-lang-ts parity` plants one disagreement and asserts `unqualified` for that project |
| W1.4 | A trace block that cannot be bound to exactly one enumerated edge is a failure, never skipped | Same suite feeds an unbindable trace block and asserts failure |
| W1.5 | A TypeScript 5 or 6 result is never reported as TypeScript 7 authority | The report's candidate table labels the SCIP artifact's compiler version; the `Status` line reflects only TypeScript 7 measurements |
| W1.9 | Every returned fact carries a basis and similar never returns inferred without a judgment backend | Context conformance query cases assert a basis on every row and no inferred rows |
| W1.10 | Propose never writes to an existing path and never marks any proposed contract with an authority other than draft | Census conformance rerun case asserts refusal; every proposed item asserted `authority: draft` |
| W1.11 | A no-match result never claims that no owner exists, and an unrelated prompt returns no-match rather than irrelevant candidates | Retrieval conformance cases assert the status and an empty candidate list for the unrelated prompt |
| W1.11 | A capped result never cuts a JSON value and always lists the omitted scope | Bounded conformance case parses the capped output and asserts the omission list |
| W1.12 | A rendered map carries the denominator and unread count of the model it renders and never omits an unresolved edge silently | Map conformance case asserts both in Mermaid and JSON |
| W1.13 | The adapter's deadline is shorter than the harness timeout and a timeout returns a small failure status | Hook conformance deadline case; the recorded timeout run in the delivery record |
| W1.13 | A skipped, killed, or distrusted hook is recorded as unavailable, never as delivery | The recorded skipped-hook run in the delivery record shows `unavailable` |
| W1.14 | No Atlas transcript or private request text appears in the committed protocol | Privacy scan row over the protocol file |
| W1.15 | retrieval.qualified is true only when the predeclared criteria were met on the held-out set | The report's per-criterion table and the lock record are compared by the acceptance item |
| W1.8 | A declared fact never acquires an observed basis and an observed fact never acquires a declared one | Model conformance declared-versus-observed cases assert two rows with distinct bases for one subject |
| W1.8 | A registry pattern that matches nothing yields a drift row on the registry | Model conformance empty-registry case asserts the drift row |
| W1.8 | A framework adapter recognizes entrypoints only where the manifest enables it | Model conformance Next.js case asserts the route entrypoint with the adapter enabled and its absence with the adapter off; `cargo test -p warrant-core` asserts no `frameworks` symbol is reachable from core |
| W1.16 | The capability report claims compiler-level references only when a pinned instrument produced them in tests | `cargo test --locked -p warrant-lang-ts references` asserts the label follows the fixture, and the fallback fixture asserts `binding` |
| W2.14 | retrieval.qualified binds the exact build, retrieval configuration, sample, criteria, and reference policy | The requalification report's build and policy digests must equal the reviewed HEAD's and the comparison policy's, and the lock record must match; acceptance item 04 compares them |
| W2.14 | The frozen-inputs record, the lock record, and the comparison inputs name identical identities | `cargo test --locked -p warrant-cli --test conformance -- qualification` refuses credit on the mismatched-policy case; W2.C1 compares the three records before approving |
| W2.1 | An undeterminable outcome never resolves to satisfied under any exception, default, or flag | Claims conformance unread-in-scope case with an exception present still yields undeterminable; observed red with the decay guard removed |
| W2.1 | An obligation's recorded basis is never stronger than the evidence that produced it | Suite asserts a declared-only obligation records `declared` even when a pattern also matches |
| W2.7 | Warrant never performs a private-key operation and never reads a key from the repository; sign renders final bytes and receives confirmation first | `cargo test --locked -p warrant-authority` asserts no `ssh-key` signing API is linked, `rule sign` without `--key` exits 2, a key placed under `warrant/` is ignored, and a declined confirmation leaves no signature file |
| W2.7 | No durable signed record is produced under the placeholder domain outside test fixtures | Authority conformance fixtures carry a `synthetic-fixture-key` marker; the lane's report lists every signed artifact in the tree and each is a fixture |
| W2.7 | Any byte change to a signed record invalidates it; verification compares bytes and never re-serializes | Authority conformance tampered-bytes cases, both verifiers |
| W2.7 | An exception-only key cannot produce a valid policy amendment | Authority conformance namespace case; `ssh-keygen -Y verify` refuses the same input |
| W2.12 | Every outcome is a raw count with a stated denominator; the protocol defines no composite score | Verify row greps for `denominator` and the coordinator reads the rubric at W2.C1 |
| W2.2 | A call occurrence never satisfies a behavior claim | Pattern conformance authorization-call case leaves the behavior link required |
| W2.2 | A registry pattern that matches nothing yields a drift finding on the registry | Pattern conformance empty-registry case asserts the drift finding |
| W2.4 | An instrument's finding set comes from its parsed report, never from its exit status | Execution conformance Fallow exit 1 and empty-report cases |
| W2.4 | A receipt for a different tree is stale and satisfies nothing; an unbound report satisfies nothing | Execution conformance stale and report-unbound cases |
| W2.4 | A bound report satisfies an evidence requirement only when its predicate holds over the canonical rows | Execution conformance forbidden-finding and failing-threshold cases assert unsatisfied while the report is bound and parsed |
| W2.6 | No signal or profile output is a scalar composite | Signals conformance asserts every numeric field is an integer count with a name |
| W2.6 | A required comparison without its base is incomplete, never zero change | Signals conformance missing-base case |
| W2.8 | Uncertain receives widening treatment everywhere the class is consumed | Identity conformance mixed-change case asserts the approval item |
| W2.8 | Equal obligation sets on one snapshot never establish restructuring | Identity conformance glob-to-file-list case asserts uncertain |
| W2.3 | A finding's id never changes when only its line moves, and never survives a change of subject | Findings conformance moved-line and replaced-occurrence cases |
| W2.3 | One wrong registration produces one primary finding with symptom counts | Findings conformance registry-mismatch grouping case |
| W2.5 | No facet reads another facet's value | Verdict conformance facet-input matrix: each facet's inputs mutated alone and only that facet changes, plus the simultaneous fail-and-incomplete case |
| W2.5 | A check result can never carry mode gate or acceptance accepted | Verdict conformance asserts the check predicate and `check-clean` on the clean fixture |
| W2.9 | A reformatted receipt fails at step 1; verification never re-serializes | Receipt conformance reformatted case; acceptance item 09 |
| W2.9 | A verified-with-unchecked result is never reported as an acceptance credential | Receipt conformance missing-artifact case asserts the result string |
| W2.9 | An unsigned receipt verifies to advisory at best | Receipt conformance unsigned case asserts `advisory`; the signed-and-recomputed case is the only one that asserts `verified` |
| W2.10 | Zero surviving mutants in core and authority at HEAD; a planted no-op mutation of the facet derivation is caught | `scripts/gate.sh` mutants stage; the lane records the planted mutation being caught |
| W2.10 | Missing authority or evidence cannot become acceptance in any adversarial case | Adversarial conformance area |
| W2.11 | Every count in the dogfood record has its denominator and no Atlas source text is committed | Privacy scan row; the coordinator reads the record at close |
| W2.13 | The reference policy and result collection stay outside the implementing agents' control | The report names the collector and the frozen digests; the coordinator checks the protocol digest matches W2.C1's approval |

## Wave map

Quoted from the dependency graph; tasks have no authored wave field. Regenerate with `plan-lint --waves` after any edit to `blocked_by`.

| wave | tasks |
|---|---|
| 1 | W0.1 |
| 2 | W0.2 · W0.8 |
| 3 | W0.3 · W0.5 |
| 4 | W0.4 · W0.7 |
| 5 | W0.6 |
| 6 | W0.G |
| 7 | W1.1 · W1.7 |
| 8 | W1.2 |
| 9 | W1.6 · W1.3 |
| 10 | W1.8 · W1.4 · W1.5 |
| 11 | W1.D10 · W1.9 |
| 12 | W1.16 · W1.10 · W1.11 · W1.12 |
| 13 | W1.13 · W1.C1 |
| 14 | W1.14 |
| 15 | W1.C2 |
| 16 | W1.15 |
| 17 | W1.G |
| 18 | W2.1 · W2.7 · W2.12 |
| 19 | W2.2 · W2.4 · W2.6 · W2.8 |
| 20 | W2.3 |
| 21 | W2.5 |
| 22 | W2.9 |
| 23 | W2.10 |
| 24 | W2.11 |
| 25 | W2.R |
| 26 | W2.14 |
| 27 | W2.C1 |
| 28 | W2.13 |
| 29 | W2.R2 |
| 30 | W2.G |

Milestones by task id: M0 is W0.x, M1 is W1.x (with W1.D10 and the two W1.C checkpoints), M2 is W2.x ending at W2.G, which is R14. The shape is intentional. M0 fans out only where crates are disjoint (snapshot beside inventory, corpus beside core) and serializes through the CLI because the conformance runner needs a binary. M1's widest wave is the model consumers (S6, S1, query) and then the query consumers (census, context, map); S7 sits behind two human checkpoints by design. M2 fans out again after obligations exist and narrows into the receipt, the gate hardening, the dogfood, and the comparison. The runtime schedules per item, so a dependency advances as soon as its prerequisites finish; the waves above are a reading aid.

## The executor experiment

Trey's question: do subscription-backed apex implementers (Astra at `low`, Sol at `xhigh` behind it) reduce the review and fix loop compared with the estate's usual Luna executors? This plan is the observation, not a controlled trial. What is recorded, per P10: for every build and spike task, the count of adopted findings at or above `major` on the first review round, the fix rounds consumed, and the route that closed the task, read from the workflow journal and `bin/delegate-audit` into `docs/acceptance/2026-M2-executor-experiment.md`. The denominator is every executable task in this plan. The comparison set is the journals of earlier plans on this estate that ran Luna executors under the same three-seat standard regime; they differ in repository, language, and reviewers, so the close report states the numbers side by side and says what they do not control for. If Astra `low` stalls or parks on a task, that is part of the record as its own row, and the retry slot's cost is counted, not hidden.

## Carried forward to the next plan

The next plan opens after W2.G and must carry, as tasks or checkpoints on Trey: D2 permanent identifiers before any durable signed record; the D5 deployment choice and its separate infrastructure authorization; D5 qualification on the real Atlas arrangement before M3 closes; S5 on macOS; the spec 17.2 rows this plan defers by name in `tests/conformance/adversarial/README.md`; the `gate` command with a protected producer; needed evidence adapters beyond vitest and Fallow; changed-source diagnostics after editing events (spec 12.6's second paragraph), which no M1 or M2 adapter delivers; framework adapters for frameworks no corpus member uses; M4's held-out comparison; and M5's transports. Nothing in this plan marks any of them complete.

## Standing constraints

From AGENTS.md and the spec, binding on every lane: no build without Trey's authorization after plan approval; agents draft rulings and never sign; no time estimates in any document; no credentials, internal hostnames, or home-directory paths in this public repository; sentence-case headings, no em dashes, no bold lead-ins, straight quotes; commits and Forgejo pushes ungated, GitHub gated; never amend, force-push, or bypass hooks; commit subjects at most 72 characters with a body stating the verification that ran.
