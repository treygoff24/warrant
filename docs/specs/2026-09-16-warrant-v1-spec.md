# Warrant v1 specification

Version: 0.3 draft, 2026-09-17. Status: incorporates Astra and Fable's co-design review and Trey's R14-R16 decisions; pending final ratification. Approval to revise is not approval to build. Owners: Trey Goff, Fable, and Astra. Fable wrote draft 0.1, Astra revised draft 0.2, and both contributed to this draft. The research trail and section numbers remain stable. The plain-language co-design summary is `docs/specs/2026-09-17-warrant-v0.3-changes.md`; the earlier revision summary remains available beside it.

## Read the vision first

If you are working on this spec, or building from it, read `docs/vision.md` in this repository first and keep it open. The spec exists to make the vision buildable; when the two disagree, the vision's four ideas win and this document gets fixed.

The vision in three sentences, for the reader who will not click: AI slop is a contract problem. Agents write slop because they are guessing about where behavior belongs, what owns it, which interface to use, and what proof will be required, and because when a check fails the agent holds the pen for the rules too. Warrant fixes that upstream with three moves: a map before the edit, a contract during, and a receipt after, with the rules signed by a human key the agents cannot reach.

Companion documents in this repository:

- `docs/design/2026-09-16-agent-builder-wishlist.md`: thirty-two wishes (W01 to W32) and six acceptance stories (A to F), written by Codex as an agent that builds Trey's software.

  It is input to scope decisions, not a promise to deliver every wish. Section 24 records what is specified, limited, deferred, or excluded.
- `docs/research/`: the research reports behind the borrowed pieces. Each is dated and says what was verified live.
- `docs/design/`: the design notes carried over from Specgate, Warrant's predecessor, including the Atlas quality-tooling research Astra wrote on 2026-09-16.

## 0. How this spec was written: the decision criteria

These criteria govern the revised design. They preserve Fable's original borrow-before-build approach while applying Trey's clarified product boundary and Astra's review.

1. The vision and Trey's rulings govern; the wish list and research inform the design. Warrant supplies architectural facts and checks, never task, planning, project, or agent management. Trey's 2026-09-17 clarification removes the vision story's earlier suggestion of tracking another lane's in-flight work (R11).
2. Borrow before build, judged by total complexity (R3, R12). Prefer an existing tool when it meets the need well and reduces implementation, integration, verification, and maintenance work. A small custom component can win when adapting a broader product costs more. Dependency count is not the objective. Section 3.3 names the borrowed foundations; section 3.4 bounds the glue Warrant owns; section 15.3 separates useful reuse from compatibility work no first user needs.
3. Fail closed wherever a verdict depends on it. "Could not analyze" is a failure of the analysis facet, never a warning that decays into a pass. Fail-open behavior exists only where a person chose it, in a profile, in writing.
4. Deterministic core, probabilistic rim. Nothing a model produced is ever an input to the compliance facet. Model output is labeled as judgment, carries its probability, and routes to a human decision or to advice. This is the line the wish list draws in W28 and the vision draws in "What it refuses to be," and every section of this spec that mentions a model repeats it.
5. Fewest nouns. A concept enters the vocabulary only if it appears in a verdict, a receipt, a query, or a policy file. Seven nouns carry the product (section 2). Where I wanted an eighth I looked for which of the seven it really was.
6. Agents are the primary user and the structured output is the API. Human-readable output is a rendering of the JSON, never a separate truth. A feature an agent cannot drive from the CLI or the MCP surface does not exist.
7. Cheap to verify beats cheap to produce. A receipt is machine-verifiable; a ruling is signed; a hard control has a fixture that fails when the control is broken. Where verification would be expensive, I chose the design that makes it cheap over the design that makes production cheap.
8. Reversible defaults now, irreversible choices only when a fixture forces them. Formats, identifiers, and directory names are chosen so that changing them later is a rename; where a choice would be hard to reverse (the receipt predicate type, the signing namespace, finding identity), it is listed for Trey to rule on in section 20.
9. TypeScript first, language-neutral by construction. The core never knows what a TypeScript import is. The first language integration proves the integration contract; the second (Rust, so Warrant can check itself) proves that the contract was not secretly TypeScript-shaped.
10. Rust first as a goal, not a rule. Where the source of truth is a Node program (the TypeScript compiler) or a Python one, a sidecar process is acceptable when it runs at check time and exits. No long-lived daemon is required to use Warrant.
11. Resolve an uncertain dependency before promising the capability that needs it. Compiler references have an early feasibility check; unsupported claims remain unsupported. Fresh evidence on the combined source is the initial rule, not a speculative evidence-reuse algorithm.
12. Say where v1 under-delivers on purpose. Section 24 marks each wish as delivered, partial, or deferred, with the reason. A partial that is not marked is a bug in the spec.
13. Write for the compiler. This spec will be compiled into a plan and a bead graph by the `writing-plans` workflow. Every requirement is numbered, testable, and phrased so a lane can close it; every milestone exit criterion says what green proves and what it does not.
14. Dogfood on Atlas, and then on Warrant. Atlas is the first customer and the frozen corpus's first real repository; Warrant checks its own repository as soon as the Rust integration exists. A rule we would not accept on our own code does not ship.
15. No time estimates anywhere in this document. Order and dependencies, yes. Durations, no.
16. Ownership is shared. Where I made a call that Trey or Astra might make differently, I wrote the alternatives and the reason I chose as I did, so that overruling me is cheap and recorded.

### 0.1 What I read and what I ruled out before writing

For draft 0.1, Fable reported reading the whole predecessor (Specgate v0.3.2: roughly 7,000 lines of Rust, 841 tests, and fourteen fixture projects), the wish list, the vision, Astra's Atlas quality-tooling and TypeScript notes, and TypeSafe's documentation. The six dated research reports are retained under `docs/research/`. Astra's revision reviewed those findings and the spec; it did not repeat all external research or run the proposed implementation spikes.

I ruled out evolving Specgate in place before writing a line of this document. The reasons are in section 23.1; the short version is that the predecessor's data model is file-level and import-centric, its baseline is a green snapshot, its policy precedence is lexicographic, its parse failures are warnings, and its command surface grew by accretion. Each of those is the opposite of a vision idea. What survives is the parser and resolver code as a starting point for the TypeScript integration, the fixture corpus as the seed of the conformance suite, and the policy-diff classifier's fail-closed rules as the seed of the widening classifier.

## 1. Product summary and scope

### 1.1 One paragraph

Warrant helps coding agents use the architecture that already exists, then checks their actual changes against human-approved contracts. Before an edit it names the owner, the interface to reuse, relevant examples, affected consumers, and the evidence acceptance will require. Afterward it reports four separate facts: whether the checked contracts were met, how much it could analyze, whether required evidence exists, and whether human approval is still needed. A receipt binds those facts to exact source and approved rules. Existing planning and execution tools own the work. Warrant owns none of it. Optional model advice cannot decide compliance; if a human deliberately makes a model question require review, that review blocks acceptance and is described as such.

What is new in Warrant is the combination: a compiler-aligned semantic model of modules, interfaces, effects, state, and capabilities; a verdict that keeps compliance, completeness, evidence, and approval apart; and rulings that are scoped, signed, expiring, and classified. What is not new, and is reused rather than claimed: signed policy and verification summaries (in-toto, SLSA VSA), findings interchange (SARIF), machine verification separated from human ratification (Gerrit), exceptions with owners and expiry (Kyverno), and agent guardrails and hooks (the harness vendors). The prior-art report (`docs/research/2026-09-16-prior-art.md`) is the reference for both lists, and public descriptions of Warrant follow it: semantic architecture verification with ratified exceptions and complete evidence accounting, not a new attestation format and not a new kind of agent guardrail.

### 1.2 Goals for v1

- G1. Agents find owners, interfaces, consumers, dependency rules, and required proof with exact references. Every answer distinguishes declarations, observations, and suggestions (W03).
- G2. Contracts use the domain's vocabulary and carry intent, ownership, authority, supported checks, and limits (W05, W07, W08).
- G3. Every verdict carries four facts that cannot collapse into one, and every integration-gate pass is backed by a receipt a machine can verify (W17, W24).
- G4. Agents draft; humans ratify signed rules and rulings. Policy changes are classified, and uncertainty receives widening treatment (W14, W15, W16).
- G5. Inventory and analysis coverage are explicit:

  unread files, unresolved constructs, and unsupported claims remain visible beside the examined denominator. No claim of universal program understanding follows from a clean result (W01, W02).
- G6. Two source revisions that each pass can produce combined source that fails.

  Warrant checks the supplied trees and distinguishes findings introduced by combination; earlier evidence remains valid only for its own source and execution inputs (W22, W23).
- G7. An existing repository can be onboarded by a census that proposes contracts from observation and keeps observed and intended architecture distinct (W26).
- G8. Taste is a profile:

  measurable preferences enter as evidence obligations, judgment preferences enter as labeled model questions, and both live in a file a user can replace (vision, "Taste is a profile").
- G9. Warrant proves its own claims:

  every hard control has a conformance fixture, the gate is mutation-tested, and instrument upgrades produce a before-and-after report on a frozen corpus before adoption (W30, W31, W32).
- G10. The first complete Atlas experience demonstrates useful guidance before editing and trustworthy checking afterward.

  A controlled comparison measures unnecessary new owners and interfaces, repair cycles, missed defects, and human review burden; fewer warnings alone is not success (sections 1.7, 17.5).

### 1.3 Non-goals

For v1 and, unless a ruling changes it, forever:

- Not a linter, formatter, test runner, package manager, workflow engine, task ledger, or account system. Those tools exist; Warrant consumes needed evidence from them (W12).
- No task, planning, project, or agent management, including features that resemble it.

  No work registration, assignments, work-overlap reports, scheduling, dependencies between repairs, progress states, or Beads-specific exports or scripts. A proposed change is an architectural query, not a task Warrant stores. A source revision supplied for comparison is code, not a registered lane (R11).
- Not an AI judge. No model output is the source of a hard rule, a compliance result, or an approval (W28).
- Not a score. There is no single number that summarizes a codebase or a change. There are contracts, evidence, findings, and unresolved obligations (W13, vision).
- Not a hosted service. Core operations run offline on the developer's machine or in CI with no network dependency (W25). The judgment lane's remote backends are optional and explicitly enabled.
- Not an architecture designer. The census proposes; a human ratifies (W26).
- Not a taint analyzer in v1. Sensitive-data contracts are declarations plus pattern recognition with stated limits, never a claim of proven data flow (W07).

Deferred from v1 with a path, not refused:

- Editor diagnostics over LSP (W21). The finding format is designed to render as LSP diagnostics; the server is a v1.x deliverable.
- Broader compiler-level TypeScript references if S1 does not qualify a usable source (W02).

  S1 runs before contracts rely on those references. The first product can ship honest binding-level checks, not a broad claim disguised by a limits paragraph.
- Full Rust source-reference analysis and languages beyond TypeScript (W04).

  A small Cargo package-dependency implementation proves the language boundary early and lets Warrant check its crate direction. Syntax-level Rust internals and additional languages follow need.
- DOT, HTML, and additional diagram exports. v1 has a scoped Mermaid diagram and the same structured model in JSON; completeness labels travel with both.
- Multiple judgment providers and a general judgment-calibration command suite.

  The initial optional advice interface stays; expanding it must earn its cost. Required human review driven by model thresholds is deferred pending D3.
- Importers and report readers not needed by the first adopter. This is a demand test, not a blanket dependency cut (sections 9.3, 15.3).
- Warrant-managed trust-root changes and scoped signer delegation. The first signer uses existing protected key administration; v1 rulings are amendments and exceptions (R15).

### 1.4 Users

- Coding agents use the CLI or MCP to read the map, run advisory checks, and draft rulings. Their existing workflows submit source to the protected gate.
- The signer. A human with an SSH key on a machine the agents do not control.

  In this estate that is Trey, signing as the `trey` user on the devbox, while agents run as `trey-agent`.
- The gate. A CI job or coordinator that runs `warrant gate` on the integrated candidate with a trust root it controls.
- Reviewers read explanations and receipts. Full semantic verification uses the dependency-light `warrant verify`; SSH tools can separately check signatures. Neither requires a model's opinion.

### 1.5 Hard constraints, binding on every change to Warrant

- HC1. No probabilistic input to compliance.

  The compliance facet uses the model, approved policy, and deterministic evidence only. Initial model output is advice. Any later model-triggered approval requirement must be separately adopted and explicitly described as blocking acceptance (D3).
- HC2. Gate acceptance requires all four facts.

  Every applicable acceptance obligation has snapshot-bound evidence or a valid signed exception to an evaluated violation, required analysis is complete, required evidence is satisfied, and no approval is pending. An exception cannot turn missing analysis, missing execution, invalid authentication, or unstable source into proof. A human can amend the requirements, with the reduced claim visible, but cannot bypass unchanged requirements into an accepted receipt. Otherwise the gate exits nonzero.
- HC3. Unread is not absent.

  A file the inventory could not read, a construct an integration could not analyze, a report that was truncated, or a required instrument that was missing sets `analysis: incomplete` and is listed with its reason.
- HC4. Every verdict has a receipt.

  A verdict without a receipt is not a verdict; the receipt binds the snapshot digest, the effective-policy digest, the inventory digest, every instrument identity, every external evidence digest, and the evaluation clock.
- HC5. Acceptance authority lives outside the implementer's control.

  The gate's executable, launcher, approved-policy anchor, trust roots, required checks, evidence-runner identities, and acceptance receipt key are protected from the agent principal, not merely stored outside the candidate. Candidate-controlled commands cannot reach them. Repository copies are documentation or proposals (sections 3.7, 11.3).
- HC6. Uncertain is widening. The policy-diff classifier reports narrowing, widening, restructuring, or uncertain, and the gate treats uncertain exactly as widening.
- HC7. No baseline snapshots.

  There is no operation that records the current findings as acceptable. Inherited debt enters as individually scoped exception rulings with owners, reasons, and expiry. HC7 forbids violation baselines, not measured floors: a mutation-score, coverage, size, or latency floor is an ordinary `evidence` requirement with a threshold in policy (section 9.3); raising the threshold is a narrowing change and lowering it is a widening one (section 11.5); and no threshold ever stands in for the acceptance of an architectural finding.
- HC8. Instruments are pinned and recorded.

  Every external tool Warrant runs or ingests is named in `warrant/instruments.lock` with its version and, where it is a binary, its digest; the receipt records what actually ran; a mismatch is `analysis: incomplete`.
- HC9. No scores. No command emits a scalar quality score for a file, module, change, or repository. Counts of specific things are fine; a weighted composite is not.
- HC10. Structured output is versioned. Every JSON document Warrant emits carries a `schema_version`, has a published JSON Schema under `schemas/`, and changes only by adding fields within a major version.
- HC11. Agent output has no decoration.

  When stdout is not a terminal, or `--format json` is given, output is JSON or NDJSON with no ANSI sequences, no progress text, and no repeated repository context.
- HC12. One conformance fixture per hard control.

  Every rule with `enforcement: static` or `pattern`, every facet transition, every exit code, and every widening class has a fixture in `tests/conformance/` with a positive case, a negative case, and a case that breaks the control and must be caught.
- HC13. Core analysis, context, check, gate, verification, and policy operations run offline.

  Optional model advice requires explicit network opt-in. The evidence executor may use the named services permitted by its approved runner profile; that is external test execution, not a hidden dependency of the core. Package installation belongs to existing tools.
- HC14. Atomic outputs. A cache entry, receipt, or model database is written to a temporary path and renamed into place; a partial write is never readable as complete.
- HC15. Filename globs scope contracts; they never express them.

  A contract's subject is a module, interface, effect, capability, or symbol; a glob may say where a module's files live and nothing else.
- HC16. Source tripwire.

  Warrant's own source is budgeted per crate in `scripts/budget.sh` and CI fails above the budget; a change that raises a budget includes an architecture note in the commit body. Budgets are alarms, not proofs.
- HC17. Every claim of completeness names its denominator. An output that says "no findings" also says how many files, modules, and contracts were evaluated and how many were not.
- HC18. Evidence cannot be promoted beyond what its check establishes.

  Each obligation has a typed claim and required evidence capabilities. A pattern occurrence cannot satisfy control flow; a known import cannot prove all effect routes; a self-reported test class cannot establish a real service run (sections 7.4, 9.2).
- HC19. All evidence credited by an authoritative gate has an authenticated producer admitted by the protected gate configuration.

  Agent-local receipts of any test class remain advisory. A file hash establishes identity, not execution provenance.

### 1.6 Definition of done for a Warrant capability

A capability is complete when: it is reachable from the CLI and, where applicable, the MCP surface; its structured output validates against its published schema; its conformance fixtures pass and its break-the-control fixture fails as designed; it has run on the frozen corpus without a regression in the corpus report; and its receipt or output names the tool build that produced it. A green unit-test run proves the unit; it does not prove the fixture, and neither proves the corpus run. Milestone exit criteria in section 21 are written in these terms.

### 1.7 The first complete experience: grouped undo in Atlas

Atlas supplies the real repository; the names below are acceptance-story examples, not assertions about paths verified in the current Atlas checkout. M0 pins an Atlas revision and records the actual symbols and tests. The complete experience has these observable results:

1. A minimal census inventories the repository and proposes the existing module boundaries. A person decides the intended owners, public interfaces, and contracts. Current behavior is not automatically approved architecture.
2. An agent receives architectural context before deciding how to implement "add grouped undo to a meeting debrief," through a verified supported harness adapter or an explicit context query (section 12.6). Warrant identifies the relevant command owner, existing undo registration and compensation interfaces, a representative existing implementation, consumers and transports, and required tests, with source references and clear evidence labels. Ambiguous ownership, unavailable delivery, or incomplete retrieval is visible; no match is not a finding that no owner exists.
3. The agent describes a proposed second undo dispatcher. Warrant shows the existing responsibility and its consumers as a reuse candidate. It distinguishes an observed interface conflict from an inferred design overlap. A legitimate new responsibility remains possible; the tool does not forbid novelty by resemblance.
4. The agent edits using its existing workflow. Warrant's advisory checks follow affected imports and registrations, name broken contracts, and state what it cannot determine. A named authorization call is only a call occurrence, never proof that unauthorized effects are prevented.
5. Existing test tools run in the approved evidence environment against the exact candidate. Warrant checks their reports and required evidence, including the real-database race, readback, authorization-denial, and failure-path cases named by the approved Atlas contracts. Each result says what the tests exercised, not that all possible behavior was proven.
6. Existing Git and coordination tools prepare the combined candidate. Warrant accepts that concrete source for checking. Any receipt for different source is stale, even if only another agent's interface changed. Fresh required evidence is collected on the combined tree; Warrant neither merges work nor schedules the rerun.
7. The protected gate checks all four facts on that tree. Trey sees what changed, what was checked, what remains uncertain, and any proposed rule change. Human approval changes the rules only through a signed ruling. Ordinary code changes under unchanged rules need no new policy signature.

The same experience must work without a model service, a task registry, or a planner integration. Readable Mermaid output is a view of the same map, not another maintained architecture document. Tests of this experience and the outcome comparison in section 17.5 are both required before claiming the product reduces supervision.

### 1.8 What belongs where

Warrant owns architectural vocabulary, inventory completeness, evidence requirements, policy comparison, the four-fact verdict, and the binding and verification of receipts. Git supplies source identity and combines branches outside Warrant. Compilers and parsers supply code facts; ast-grep supplies structural matches; existing tests, Fallow, coverage and mutation tools supply their own measurements. An existing isolated runner executes candidate-controlled commands. SSH tools sign records. SQLite stores the queryable map. Planning skills, Beads, and agent workflows decide and track work without a Warrant-specific integration.

The first product includes the full path in section 1.7, not every possible adapter. Sections describing future-compatible formats do not turn optional expansions into release prerequisites. Section 21 states the dependency order for building Warrant itself; it defines no planning feature inside the product.

### 1.9 The four opening examples and what helps detect them

The vision's opening failures need different mechanisms. A green structural check does not establish that none of them exists.

| Example | Before the edit | After the edit | Remaining limit |
|---|---|---|---|
| A second cache | Context and proposal retrieval name an existing owner or interface as a reuse candidate. | A new module, store, or public symbol can trigger a conceptual-expansion signal. | Duplication inside allowed boundaries need not violate a structural contract or create a signal. Different names can defeat retrieval; S7 measures those misses. |
| A third validator | Declared responsibilities, aliases, and same-shape matches suggest existing implementations. | Conceptual-expansion signals and relevant borrowed duplicate-code evidence can prompt review. | Same shape is not same purpose, and different code can serve the same purpose. Neither resemblance nor a count proves duplication. |
| A helper nobody calls | Context can show existing consumers and interfaces, but cannot know who will consume unwritten code. | Borrowed unused-code evidence, such as an applicable Fallow or knip result, is consumed through an evidence requirement. | Only the detector's supported symbols and entrypoints are covered. Dynamic or unresolved consumers can invalidate an unused-code inference. |
| `authorize` never on the request path | Context names the capability's required authorization behavior and evidence, not just a function to mention. | Structural checks report call occurrences; named denial and failure tests help detect behavior that bypasses the intended control. | Binding-level references do not prove guarding control flow. A passing test establishes only its exercised cases; test adequacy remains unproved. |

Ordinary dependency, interface, effect, and state contracts catch supported boundary violations, not every form of conceptual duplication. Evidence and capability contracts connect the other checks to explicit obligations. Retrieval qualification (S7), actual before-decision delivery (section 12.6), and the early outcome checkpoint (R14) test the upstream promise. Passing Warrant is never a claim that the code contains no slop.

## 2. Domain vocabulary

Seven nouns carry the product. Everything else in this document is defined in terms of them.

Snapshot. An exact, content-addressed set of files. A snapshot has a kind (`worktree`, `index`, `commit`, `tree`), a tree identity computed the same way for every kind, and a repository identity. Snapshots are immutable; changed source is a different snapshot. A combined candidate is simply another supplied snapshot, not a Warrant-managed work item. Section 4.

Inventory. The classified list of every path in a snapshot and every path deliberately excluded from it, each with a class, the rule that assigned the class, and a reason. The inventory is the denominator of every claim Warrant makes. Section 5.

Program model. What the build believes about the code: modules, files, symbols, edges (imports, re-exports, dynamic loads, registrations, declared links), entrypoints, effects, and a capability report per language integration stating what it could and could not resolve. Stored as one SQLite database per snapshot. Section 6.

Contract. A declaration of architectural intent in the domain's vocabulary, with a kind (`module`, `dependency`, `interface`, `effect`, `state`, `capability`, `pattern`, `evidence`, `data`), an intent, an owner, an authority, typed evidence requirements, an enforcement mode, and stated limits. Canonical rules have a stable policy digest. Applying them to a snapshot produces a separate obligation-set digest. Section 7.

Obligation and evidence. An obligation is one concrete checkable requirement that a contract produces when applied to one subject in one snapshot ("`billing` must not import `mail/adapter`"; "capability `undoable-operation` requires a readback test receipt for `debrief.undo`"). Evidence is whatever satisfies or fails it: a graph observation, a pattern match, an external receipt bound to the snapshot, or a valid exception ruling. Sections 8 and 9.

Finding and verdict. A finding is an unsatisfied obligation with an explanation that answers the seven questions in W19. A verdict is the result of evaluating a snapshot under an effective policy: four facets (compliance, analysis, evidence, approval), the findings, reviewable signals, and a derived acceptance state. Section 10.

Receipt. The verifiable record of a verdict: what was checked, against which exact inputs, by which tool build, with what result, at what clock. A receipt can be verified by a machine with no access to a model. Section 10.

Supporting terms, each defined where it is first used and collected here for lookup:

- Ruling:

  a signed record by which a human changes the policy, grants an exception, changes a trust root, or overrides a gate, with scope, policy digest, candidate range, expiry, and supersession. Section 11.
- Exception: a ruling of kind `exception` that satisfies a specific obligation for a specific finding identity, with a disposition (`false-positive`, `accepted-design`, `temporary-debt`). Section 11.4.
- Profile: a file that carries taste: measurable preferences as evidence obligations and judgment preferences as typed questions with thresholds. Section 16.
- Instrument: an external tool whose output Warrant runs or ingests, identified by name, version, and digest. Section 9.
- Base and combined candidate: concrete source revisions supplied for comparison. Their creation and any work coordination belong to external tools. Section 13.
- Effective policy: canonical rules with explicit defaults and declarations, independent of which subjects happen to exist today. Snapshot application is separate. Section 7.6.
- Widening, narrowing, restructuring, uncertain: the four classes of policy change. Section 11.5.
- Capability: a contract kind that links a user-visible feature's entrypoint, authorization, domain operation, persisted effect, readback, failure path, and tests. Section 7.4.
- Entrypoint: a file or symbol the framework, runtime, operator, or an external consumer can invoke without an ordinary import from first-party source. Section 5.4.
- Effect: an operation that changes the world outside a pure computation, declared as an interface with permitted callers. Section 7.4.
- Judgment: a typed answer from a model to a question in a profile, with a probability, labeled as a model's opinion. Section 16.
- Signal: a reviewable indicator attached to a verdict that is neither a finding nor a facet, such as a metric-gaming signal. Section 8.5.
- Census: the onboarding operation that proposes contracts from observation. Section 15.
- Subject: the thing an obligation applies to: a module, file, symbol, edge, or capability.

## 3. Architecture

### 3.1 Shape

Warrant is one binary, `warrant`, built from a Cargo workspace of small crates with a declared dependency direction. The workspace exists because crate boundaries are the strongest module boundaries Rust offers and because Warrant's own Rust integration (section 6.7) will check them. The crates:

```
warrant/
  crates/
    core/         nouns, policy compiler, obligation evaluation, verdict, receipt, verifier, widening classifier
    snapshot/     git tree reads, worktree hashing, source comparison, rename detection
    inventory/    classification rules, entrypoint discovery, generated and vendored provenance
    model/        SQLite schema, writers, read-only query surface
    lang-ts/      TypeScript and JavaScript integration (oxc, oxc_resolver, tsc parity, tsgo sidecar)
    lang-rust/    Cargo units and package dependencies; small second implementation proving the seam
    evidence/     attest wrapper, SARIF and JSON reporter ingest, evidence kinds, instrument pinning
    authority/    ruling records, canonicalization, sshsig signing and verification, trust roots
    judgment/     optional typed advice; one needed provider, no required calibration platform
    census/       observation and proposals; policy importers only where adoption needs them
    render/       map rendering (Mermaid, JSON), human output
    cli/          clap commands, MCP server, hook adapters, output formatting
  schemas/        JSON Schema for every emitted document
  tests/conformance/  fixtures per hard control
  tests/corpus/   frozen corpus manifest and tooling (the corpus itself lives outside the repo)
  warrant/        Warrant's own policy, rulings, profiles, instruments.lock
  docs/
```

Dependency direction: `core` depends on nothing else in the workspace. `snapshot`, `inventory`, `model`, `evidence`, `authority`, `judgment`, `census`, and `render` depend on `core` only. `lang-*` depend on `core` and `model`. `cli` depends on everything. Nothing depends on `cli`. This is Warrant's first `dependency` contract, in `warrant/policy/workspace.yaml`, and it is enforced by CI from the day the Rust integration exists and by a `cargo metadata` script before that.

### 3.2 The pipeline

Every command is a walk along the same pipeline, stopping where the command's result is complete. Each stage produces a content-addressed artifact keyed by the digests of its inputs, so a cache hit is a key match and an explanation of reuse is the list of matching keys.

```
snapshot ──► inventory ──► program model ──► evaluation ──► verdict + receipt
   │             │               │                ▲               ▲
   │             │               │                │               │
   │             │               │         effective policy    evidence
   │             │               │      (contracts compiled,   (receipts,
   │             │               │       conflicts checked,     SARIF, exceptions,
   │             │               │       digest computed)       judgments)
   │             │               │
tree id     inventory digest   model digest
```

- `warrant snapshot` stops after the first stage and prints the tree identity.
- `warrant inventory` stops after the second.
- `warrant model` and `warrant query` stop after the third; `query` opens the SQLite database.
- `warrant check` and `warrant gate` run to the end; `check` is advisory, `gate` evaluates supplied source under protected acceptance configuration.
- `warrant context`, `warrant propose`, and `warrant explain` read the model and the effective policy and produce answers without producing a verdict.
- `warrant verify` reads a receipt and recomputes what it can without re-running instruments.

### 3.3 Seams and the stitch map

Each seam names the borrowed tool, the reason, and the alternatives that lost. The versions to pin are in section 19 and were verified by the research lanes on 2026-09-16.

| Seam | Borrowed | Why | Rejected |
|---|---|---|---|
| Snapshot identity and comparison | Git object model through `gix`; temporary-index Git commands for worktree capture until S3 qualifies native capture | Git already content-addresses trees; all source kinds can be compared. External Git tooling supplies the combined tree | Own hashing; modification-time identity; a Warrant-owned merge workflow |
| TypeScript syntax and bindings | `oxc_parser`, `oxc_ast`, `oxc_semantic` | Rust-native, fast, gives per-file symbols, scopes, references, and export and import bindings; already proven in the predecessor | tree-sitter for TypeScript (no binding resolution); SWC (heavier, less semantic surface); the TypeScript compiler as the only parser (Node dependency on every check) |
| Module resolution | `oxc_resolver`, qualified against `tsc --traceResolution` on the frozen corpus | Handles tsconfig paths, package exports, conditions, symlinks; parity with the compiler is measured, not assumed | Own resolver (the predecessor's early mistake); tsc-only (slow, Node on the fast path) |
| Compiler-authority symbol references | A small Node indexer over the exact `typescript@7.0.2` package's `typescript/unstable/async` client, emitting a Warrant-owned, versioned canonical index; the native compiler's LSP as the fallback; the published SCIP artifact as a third candidate; decided by spike S1 | The compiler is the only source that knows type-derived references; TypeScript 7 ships no supported programmatic API, so whatever surface wins is exact-pinned and stays behind a spike | Reimplementing type inference (no); a hypothetical `@typescript/api` package (no such stable surface exists as of 2026-09-16); treating `unstable/*` exports as stable |
| Structural pattern contracts | ast-grep rules through the `ast-grep-core` and `ast-grep-config` crates over tree-sitter grammars | A pattern-enforced contract is an ast-grep rule with Warrant metadata; users already know the YAML | Semgrep (Python, licensing of the engine for embedding); CodeQL (license); hand-rolled matchers |
| Evidence from other tools | SARIF through `serde-sarif`; qualified readers for the first repository's test, Fallow, type, coverage, and mutation reports. Section 9.3 governs additional readers | Preserve tool meaning and verify provenance; reuse shared readers when that reduces total work | Rebuilding detectors; committing to every reporter before a consumer needs it |
| Receipts and rulings | in-toto Attestation Statement v1 with one Warrant predicate type per kind; RFC 8785 canonical JSON; detached SSH signatures (sshsig) under one namespace per kind; signing only ever by `ssh-keygen`, verification in-process with the `ssh-key` crate and `ssh-keygen -Y verify` as the oracle | An existing schema and store convention, human-readable, works with the key Trey already has; the trust root's own `namespaces=` matching gives kind restrictions for free. Verification is Warrant's own with `ssh-keygen -Y verify` as the oracle: the Statement is a schema, not a verifier, and a detached sshsig is not a DSSE or Sigstore bundle (section 11.2) | GPG (UX); Sigstore keyless (identity infrastructure; a later option for teams); signed commits as the approval channel (approves a tree, not a scoped statement) |
| Model store and query surface | SQLite through `rusqlite` (bundled), opened read-only for queries with an authorizer | One file per snapshot, every agent speaks SQL, no server | In-memory only (no query surface); a graph database (weight); JSON dumps (no queries) |
| Policy shape | YAML with a published JSON Schema, validated with `schemars`-generated schemas | Diffable, signable, readable by agents and humans; the schema is the lint | Rego (a second language); Cedar (permission model does not fit graph obligations; kept as a candidate for widening analysis); a TypeScript DSL (harder to sign and diff); CUE and Dhall (adoption) |
| Optional model advice | System One's state-plus-typed-questions shape; a needed backend behind a small adapter | Keeps model suggestions structured and separate from observed facts. Usefulness and calibration are unproven until measured | A required hosted service; a broad provider platform before useful advice is demonstrated |
| CLI, output, errors | `clap`, `serde_json`, `schemars`, `miette` for human rendering | Standard | Anything custom |

### 3.4 What Warrant writes itself

This is the whole list. A component under `crates/` that is not an instance of one of these is scope creep and should be questioned in review.

1. The contract vocabulary, the policy compiler that produces an effective policy, and the policy linter (section 7).
2. Obligation derivation and evaluation over the program model (section 8).
3. The inventory classifier and completeness accounting (section 5).
4. The language integration contract and its capability report (section 6.1), and the TypeScript and Rust integrations built on borrowed parsers and resolvers.
5. The four-facet verdict, the receipt format, and the verifier (section 10).
6. The ruling record format, the signing and verification glue, and the exception lifecycle (section 11).
7. The policy-widening classifier (section 11.5).
8. The evidence binder: the attest wrapper, evidence kinds, SARIF and reporter mapping to obligations, instrument pinning and upgrade reports (section 9).
9. The profile format and optional typed-advice adapter (section 16); expanded calibration and provider support are deferred.
10. The census proposer and any configuration importer actually needed for adoption (section 15).
11. The map renderer and human output (section 14).
12. The CLI, MCP server, and hook adapters as projections of the same operations (section 12).
13. The conformance harness and corpus tooling (section 17).

### 3.5 Directories and stores

In the repository being checked:

```
warrant/
  warrant.yaml          manifest: schema version, integrations enabled, policy and profile paths, gate settings
  policy/*.yaml         contracts, one or more files, all compiled together
  rulings/<id>.json     signed ruling records
  rulings/<id>.json.sig detached sshsig signatures
  profiles/*.yaml       taste profiles; the manifest names the active one
  instruments.lock      pinned instrument identities
  allowed_signers       a documentation copy of the trust root; never the verification input (HC5)
```

The directory is visible, not hidden, because the contract is meant to be read by agents that list a repository, and because a hidden directory reads as tooling rather than as architecture. The name is decision D1.

On the machine running Warrant:

```
${XDG_CACHE_HOME:-~/.cache}/warrant/<repo-id>/
  snapshots/<tree-id>/<analysis-key>/  inventory and model artifacts keyed by all analysis inputs
  evaluations/<eval-key>/         evaluation records; acceptance and expiry rechecked each invocation
  receipts/<receipt-id>.json      receipts produced here
${XDG_CONFIG_HOME:-~/.config}/warrant/
  allowed_signers                 the gate's trust root on this machine, if the gate runs here
  revoked_keys                    optional; OpenSSH revoked-keys format
  config.toml                     jobs, judgment backends (credentials by reference only), defaults
```

Nothing under the repository's `warrant/` is trusted for approval without signature verification and comparison to the protected approved-policy anchor. An analysis key includes source, dependency artifacts, relevant configuration, declarations, integration and adapter identities, and platform features that affect analysis. Tree identity alone is insufficient. An agent-writable cache is untrusted input: the authoritative gate recomputes it or verifies an admitted producer's binding to all inputs. Hashing fabricated rows does not make them genuine observations.

### 3.6 Process and resources

Warrant runs as a single process that exits. Parallelism is a bounded `rayon` pool sized by `--jobs`, defaulting to half the available cores and never fewer than one, because a fleet of agents shares the machine (W25). SIGINT and SIGTERM cancel cooperatively: in-flight artifacts are discarded, nothing partial is renamed into place (HC14), and the exit code is 130 or 143. Memory is bounded by streaming file reads and by writing the model to SQLite as it is built rather than holding a whole-repository graph in memory. External instruments run as child processes with a wall-clock limit from `instruments.lock`, with stdout and stderr captured to files whose digests go into the receipt.

### 3.7 Trust boundaries

Four roles matter. They may use existing CI or local isolation; Warrant does not build a runner platform. The agent side is untrusted: source, tests, configuration, reports, and local tools may be edited or fabricated. Candidate code still runs on the execution side, so moving it to another account alone does not make its reports or that account's credentials trustworthy. The protected controller must observe execution without exposing its credential or acceptance authority to those commands.

- The agent controls its own working environment.

  It can edit source, policy proposals, caches, and JSON; it can run commands as its own user. It cannot produce a valid signature without the corresponding key.
- The signer holds the human ruling key and signs canonical records after reading an effect summary.

  The signing tool is independently installed and trusted. It never loads candidate plugins or executes repository commands. Agents never reach this key.
- The evidence executor runs candidate-controlled tests and instruments against materialized source in an existing isolated environment.

  It has neither the human key nor authority to alter the gate configuration. A protected runner controller observes execution and authenticates evidence; candidate code cannot use its credential to fabricate attestations.
- The gate evaluates supplied source with a protected executable and launcher, approved-policy anchor, required-evidence configuration, trusted evidence producers, and clock.

  It verifies rather than trusts candidate-provided evidence fields. Its receipt signing credential is unavailable to candidate code. The system accepting the change consumes this authenticated receipt for the exact candidate and configuration, never an implementer's arbitrary exit zero.

The approved-policy anchor is an independently supplied record naming the admitted policy and instrument/configuration identities. The gate verifies the signed chain from that anchor to any proposed replacement. A candidate's `--base` value, repository trust-root copy, or edited CI configuration cannot select the authority against which it is judged. D5 chooses the concrete deployment before an authoritative Atlas gate can be claimed.

A candidate first deployment uses an existing isolated worker launched by a protected controller. The controller runs no candidate code, holds the evidence-producer credential, materializes approved inputs, captures results, and authenticates them. The worker has no signing credential, reads immutable source, and can reach only admitted inputs and services. The protected gate may share the controller's trusted installation, while the human signing key remains separate. A human-managed service plus an existing container runner is one candidate, not a selected or proven estate deployment. D5 qualification checks the actual boundary, not the number of accounts or the presence of a read-only mount. Creating accounts, services, or infrastructure needs its own human authorization after the R14 checkpoint.

Residual limits: a compromised trusted controller or signer, a signer who approves without understanding, and a faithfully executed test that does not test the intended behavior. Merely putting a trust-root file under another user's ownership does not protect a gate running arbitrary agent commands as the agent's own user. Such a deployment is advisory, not authoritative.

## 4. Snapshots

A snapshot answers "which exact bytes did Warrant look at," and every other artifact hangs off its identity. The design borrows git's object model wholesale: a snapshot is a git tree, whatever it was made from.

### 4.1 Kinds

- `commit`: the tree of a revision. `warrant snapshot --commit <rev>`.
- `index`: the staged tree, exactly what `git write-tree` would produce. `warrant snapshot --index`.
- `worktree`: the working tree, including untracked files that are not ignored, hashed into a temporary in-memory index and written as a tree.

  The repository's real index is never touched. `warrant snapshot --worktree` (the default when no kind is given).
- `tree`: an existing tree object supplied by another tool, including a combined candidate. `warrant snapshot --tree <oid>`.

Warrant does not merge branches or resolve conflicts. An unmerged index or a missing supplied tree is an input error, exit 2, with no acceptance receipt. External Git tools decide how to produce the candidate (W22).

### 4.2 Identity

A snapshot identity is `{object_format}:{tree_id}`, where the object format is the repository's (`sha1` or `sha256`) and the tree id is the git tree object id. Two snapshots with the same identity contain the same bytes at the same paths with the same modes; this is git's guarantee and Warrant adds nothing to it. The repository identity is the object id of the root commit (the lexicographically first if there are several), so that caches and receipts from different clones of the same repository are comparable.

Repositories without git are not supported in v1. The vision commits to git as the snapshot store, and a directory without history has no base to integrate against and no identity a receipt can name. `warrant snapshot` on a non-repository exits 2 with an explanation.

### 4.3 What a snapshot excludes, and how the exclusion is recorded

- Worktree capture omits ignored untracked files and records exclusions and ignore-configuration digests.

  Already tracked files remain in the tree even if an ignore rule matches them. Commit, index, and supplied-tree reads use the exact object-store entries; local ignore rules cannot remove them. A historical tree cannot reveal ignored files that never entered it: the omitted-file count is `unknown` unless a capture manifest supplies it. Required generated or dependency artifacts have separate digests (section 9.2).
- Submodules appear as their commit id and are not descended in v1. A contract whose scope touches a submodule path produces `analysis: incomplete` with reason `submodule-not-descended`.
- Files above the size cap (`warrant.yaml: snapshot.max_file_bytes`, default 8 MiB) are recorded in the inventory as `unread: oversize`. Binary files are recorded and never parsed.
- Symlinks are recorded as symlink entries. A symlink whose target is inside the snapshot is followed by integrations; one whose target is outside is `unread: external-symlink`.
- Two paths that differ only by case are an error on any platform (`analysis: incomplete`, reason `case-collision`), because the same snapshot would resolve differently on different filesystems.

### 4.4 Reading files from a snapshot

Committed and staged content is read from the object store by blob id. Worktree content is read from disk and its blob id is verified at read time; if a file's content no longer matches the id computed when the snapshot was taken (another agent wrote it in between), the read fails, Warrant retakes the snapshot once, and if it changes again the command exits with `snapshot-unstable` and no artifact. This is how W23's "do not consume half-written files from another agent" is met: a snapshot is either exactly what was hashed or it is not a snapshot.

### 4.5 The snapshot manifest

Every artifact downstream carries this record, and the receipt embeds it:

```json
{
  "schema_version": "warrant.snapshot/1",
  "repo": "sha1:9f2c...",
  "kind": "tree",
  "tree": "sha1:71ab...",
  "object_format": "sha1",
  "commit": null,
  "capture": { "kind": "supplied-tree", "manifest_digest": null },
  "excluded": { "ignored_files": null, "ignored_count_reason": "not recoverable from tree", "submodules": 1, "oversize": 0 },
  "taken_at": "2026-09-16T21:04:11Z"
}
```

### 4.6 Considered and rejected

- Warrant's own content hashing scheme. It would not be comparable to commits, could not use `merge-tree`, and would reimplement what git does with thirty years of testing.
- Change detection by modification time. It lies under `git checkout`, under `touch`, and under a second agent's editor.
- Descending into submodules in v1. Correct handling needs per-submodule integrations and identities; deferring it is honest as long as the inventory says so, which it does.

## 5. Inventory

The inventory is the denominator. Every claim Warrant makes about a snapshot is a claim about the files in the inventory, and every file the inventory could not classify or read is listed as such. The inventory is what makes "clean because a tool skipped it" (story C) impossible to state without lying.

### 5.1 Entries

Every path in the snapshot, and every path deliberately excluded from it, has one entry:

```json
{
  "path": "apps/worker/src/workflows/undo.ts",
  "blob": "sha1:...",
  "class": "source",
  "language": "typescript",
  "unit": "apps/worker",
  "module": "worker/workflows",
  "by": "rule:module.worker/workflows.files",
  "reason": "matched files glob apps/worker/src/workflows/**",
  "entrypoints": [ { "kind": "workflow", "basis": "declared", "by": "registry:dbos-workflows" } ],
  "unread": null,
  "generated_by": null,
  "vendored_from": null
}
```

Classes: `source`, `test`, `config`, `script`, `migration`, `schema`, `generated`, `vendored`, `asset`, `doc`, `build-output`, `submodule`, `ignored`, `unknown`, `unread`. A file has exactly one class. `by` names the rule that assigned it, and `reason` is the human sentence.

### 5.2 Classification rules and their order

Classification and ownership are separate. Class defaults come from each integration (`*.test.ts` and `__tests__/**` are `test`, `*.d.ts` is `schema`); explicit class declarations or per-path overrides must name any default they replace. Conflicting explicit class assignments are a lint error. A module's `files` selector assigns ownership, not class: a colocated test remains a test inside its owning module. Overlapping module ownership is a separate lint error. Neither path order nor file order resolves ambiguity.

A file whose extension belongs to an enabled integration and that no rule classifies is `source` with `module: null`, and an unowned source file fails the completeness obligation `inventory.owned` (W01: a new source file must not fall outside policy because nobody updated a glob). A file no rule classifies and no integration claims is `unknown`, listed, and does not by itself block acceptance; the manifest can raise `unknown` to blocking (`inventory.unknown: block`).

### 5.3 Generated and vendored files

A `generated` entry carries a producer: the command or instrument that makes it and the inputs it reads. `warrant inventory --verify-generated` re-runs producers marked `reproducible: true` in a temporary directory and compares blob ids; drift is reported as `generated-drift` and, when a contract requires reproducibility, is a finding. Generated files that are gitignored and therefore absent from the snapshot are recorded as `generated-absent` with their producer, and an integration that needs them (an import that resolves into a generated path) reports the unresolved edge with that reason rather than "module not found."

A `vendored` entry carries its source (package or URL and version) and its treatment: `excluded-from-contracts` (the default: not a subject of ownership or dependency contracts, still resolved as a target so that "who imports the vendored thing" is answerable) or `checked-as-third-party` (treated exactly like a package dependency for dependency contracts).

### 5.4 Entrypoints

An entrypoint is a file or symbol that something outside first-party source can invoke: the framework, the runtime, an operator, a scheduler, or an external consumer. Missing one makes live code look dead; inventing one hides dead code. Each entrypoint carries a kind, a basis (`declared` or `observed`), and the rule or declaration that produced it.

Kinds: `package-bin`, `package-main`, `package-export`, `framework-route`, `test`, `script`, `migration`, `registry` (a symbol registered through a declared registry, such as an MCP tool table or a workflow registration call), `cli-command`, `workflow`, `external-consumer` (declared: a consumer outside this repository that depends on a symbol or route), `config-referenced` (a file named by a configuration file that a declared loader reads).

Sources, in v1: `package.json` (`bin`, `main`, `module`, `exports`); `registry` declarations in policy (section 7.5), which say "symbols passed to `registerTool` in `packages/core/src/tools/registry.ts` are entrypoints of kind `registry`" and are matched as ast-grep patterns; explicit `entrypoints:` declarations in `warrant.yaml`; and integration adapters for the frameworks the corpus uses (file-system routes for Next.js when the manifest enables the adapter). No framework knowledge is built into the core; adapters are small, named, and versioned as instruments.

### 5.5 Completeness accounting

The inventory summary is part of every verdict and every receipt:

```json
{
  "files": 1834,
  "by_class": { "source": 912, "test": 388, "config": 41, "generated": 12, "vendored": 3, "doc": 77, "asset": 390, "unknown": 9, "unread": 2 },
  "unread": [ { "path": "apps/web/public/big.bin", "reason": "oversize" }, { "path": "tools/link", "reason": "external-symlink" } ],
  "unowned_source": [],
  "unknown": [ "scripts/legacy.pl", "..." ],
  "ignored_files": 4211,
  "submodules": [ { "path": "vendor/atlas-old", "commit": "sha1:..." } ]
}
```

The analysis facet is `incomplete` when any of these hold, each with a code the verdict lists: an `unread` file that a contract's scope covers (`unread-in-scope`); an `unowned_source` file (`unowned-source`); a case collision; an unstable snapshot; a submodule in a contract's scope; an integration reporting an unsupported construct in a file a contract's scope covers (`unsupported-construct`); an unresolved dynamic load in a module that has not declared how dynamic loads are handled (`unresolved-dynamic`); a required instrument that did not run or whose identity did not match the lock (`instrument-missing`, `instrument-mismatch`); a truncated or malformed required report (`report-truncated`). None of these is a warning. Each is listed with the path or instrument and the sentence that explains it.

### 5.6 Workspaces, units, aliases, symlinks, case

Units (compilation or package scopes) are discovered from workspace manifests: `package.json` workspaces, `pnpm-workspace.yaml`, the nearest `tsconfig.json` per file with project references honored, `Cargo.toml` workspaces. Every source file belongs to exactly one unit, recorded on the entry, and integrations resolve within the unit's configuration. Path aliases and package `exports` maps are resolved by the integration, and the inventory records which alias table applied. A nested git repository that is not a submodule is an error in v1 (`nested-repository`), because its files have two identities.

### 5.7 Proofs

The conformance fixtures for this section (section 17) include: a first-party file added outside every module's `files` selector, which must fail `inventory.owned` while everything else passes; a declared registry entrypoint removed from the source, which must produce a finding on the capability that relied on it; a required file made unreadable, which must set `analysis: incomplete` with `unread-in-scope`; two classification rules that overlap, which must fail policy lint; and a generated file with drift, which must be reported as `generated-drift`.

### 5.8 Considered and rejected

- First-match-wins classification. It is how the predecessor's globs worked and it is why a permissive rule could win by filename order (W06).
- Treating unknown files as source by default. It would make every stray script a completeness failure and train people to exclude directories, which is the W13 gaming pattern.
- Building framework knowledge into the core. Every framework adapter is an instrument with a version, so that a Next.js convention change is an instrument change, not a silent inventory change.

## 6. Program model

The program model is what the build believes. It is built by language integrations behind one contract, stored in SQLite, and queried by agents directly.

### 6.1 The integration contract

A language integration is a Rust crate implementing five operations and reporting its capabilities honestly:

- `discover(snapshot, inventory) -> units`: the compilation or package units it will analyze, with their configuration files.
- `analyze(unit) -> files, symbols, references, edges, unsupported`:

  per-file symbol tables, exported bindings, import and re-export bindings, in-file references to imported bindings, dynamic-load sites, and every construct it could not analyze with a reason.
- `resolve(edge) -> target file | external package | unresolved(reason)`: module resolution under the unit's configuration.
- `entrypoints(unit) -> entrypoints`: what the integration can observe (package manifests, test files, framework adapters it ships).
- `capabilities() -> CapabilityReport`.

The capability report is embedded in the model, printed by `warrant model --capabilities`, and copied into every receipt:

```json
{
  "integration": "lang-ts",
  "version": "0.1.0",
  "instruments": { "oxc_parser": "0.150.0", "oxc_semantic": "0.150.0", "oxc_resolver": "11.24.3" },
  "resolution_oracle": "typescript@7.0.2 --traceResolution",
  "compiler_reference_instrument": null,
  "resolution_authority": "parity-qualified",
  "resolution_modes_qualified": ["bundler"],
  "symbol_level": "binding",
  "type_only_distinction": true,
  "supports": ["esm-import", "esm-reexport", "cjs-require-literal", "dynamic-import-literal", "tsconfig-paths", "package-exports", "package-imports", "project-references"],
  "unsupported": [
    { "construct": "dynamic-import-nonliteral", "treatment": "unresolved-dynamic" },
    { "construct": "reflection-registration", "treatment": "requires-declaration" },
    { "construct": "type-derived-reference", "treatment": "not-observed" }
  ],
  "limits": "Consumers of a symbol are files that import its binding directly or through re-export chains. References that arise from type inference, dependency injection, or reflection are not observed."
}
```

`resolution_authority` is `native`, `parity-qualified`, `unqualified`, or `syntax-only`. Native names the compiler/build-system source of the particular fact; parity-qualified names measured agreement for specific configuration features on the frozen corpus. Neither implies compiler-level references. `resolution_oracle` names the qualification invocation, while `compiler_reference_instrument` names the reference producer or is null. Qualified modes appear only after S6's measured result, never from a dependency's advertised feature list. `symbol_level` is `none`, `binding`, or `compiler`.

An unqualified requirement makes analysis incomplete. A typed `accepted-limitation` amendment may explicitly narrow the acceptance requirement for a named feature, but it does not turn the unsupported claim into satisfied evidence. The original gap, the signed disposition, expiry, and the reduced denominator remain visible. Full analysis means complete for those declared requirements, never that the accepted gap was technically solved.

### 6.2 Storage

One SQLite file per snapshot at `snapshots/<tree-id>/model.sqlite`, written once and then opened read-only. Tables:

- `meta(key, value)`: snapshot manifest, model digest, tool build, integration versions.
- `units(id, integration, root, config_path, kind)`.
- `files(id, path, blob, class, language, unit_id, module_id)`.
- `modules(id, name, declared_by, intent, owner)`: from `module` contracts, or from census proposals when running in census mode.
- `symbols(id, file_id, name, kind, exported, export_name, visibility, span_start, span_end)`.
- `edges(id, kind, from_file, from_symbol, to_file, to_symbol, to_external, resolved, unresolved_reason, type_only, span_start, span_end, basis)`: kinds `import`, `reexport`, `dynamic_import`, `require`, `registration`, `declared`, `config_reference`.
- `references(id, file_id, symbol_id, span_start, span_end)`: in-file uses of imported bindings.
- `entrypoints(id, file_id, symbol_id, kind, basis, declared_by)`.
- `effects(id, name, symbol_id, declared_by)`.
- `unsupported(id, file_id, construct, span_start, span_end, reason)`.
- `capability_reports(integration, json)`.

Views: `module_edges(from_module, to_module, edge_count, type_only_count, first_edge_id)`, `symbol_consumers(symbol_id, consumer_file, via_reexport_chain)`, `module_cycles(cycle_id, module_id, position)` materialized at build time from the condensation of the module graph, `unowned_files`.

The model digest is the sha256 of a canonical dump of semantic rows in a fixed order, excluding its own digest and incidental timestamps. Equal source, policy declarations, dependency artifacts, analysis configuration, platform semantics, and tool identities must produce equal model bytes. A mismatch is a determinism failure to investigate, not proof that only the instrument changed (W31).

### 6.3 Modules

Modules are the unit of architectural intent. They are declared in `module` contracts (section 7.4) with a `files` selector (HC15), or proposed by the census. Every source file belongs to at most one module; overlapping selectors are a policy lint error; an unassigned source file is unowned (section 5.2). A module's public interface is the set of its exported symbols that its `interface` declaration names, or, when no declaration exists, the set of symbols imported from it by files outside the module (an observed interface, labeled as such).

### 6.4 Edges and the dynamic forms

Edges are recorded with their basis. Static ESM imports and re-exports, CommonJS `require` with a literal specifier, and `import()` with a literal specifier resolve normally. Type-only imports (`import type`, `import { type X }`) are recorded with `type_only = 1` and are excluded from dependency contracts unless the contract says `include_type_only: true`.

A dynamic load whose target cannot be determined statically is recorded as `unresolved` with reason `dynamic-nonliteral` and listed under the file's unsupported constructs. The obligation this creates is on the file's module: either a `declared` edge exists (a `dependency` contract's `declares:` entry naming the target, reviewed and signed like any contract) or the module's contract sets `dynamic_loads: unresolved-allowed`, which is a widening-classified setting. With neither, the analysis facet is incomplete (`unresolved-dynamic`). Registrations, dependency injection, and reflection follow the same rule through `registry` declarations (section 7.5): what a declaration says is treated as declared evidence and labeled as such; nothing is inferred from a function's name.

### 6.5 The binding-level symbol graph

The initial TypeScript integration builds a binding-level graph from oxc: exported bindings, resolved import and re-export bindings, in-file references to imported bindings, and dynamic-load sites. It answers which observed files import or reference a binding. It does not prove execution order, all possible callers, type-derived references, or that a check guards an effect. Framework registration can be observed only through a declared and qualified recognition rule. Required evidence capabilities are machine-checked (section 7.4); writing `limits: binding-level` does not permit a stronger claim to pass. S1 is resolved before a contract depends on compiler references, and even compiler references are not a control-flow proof.

### 6.6 Spike S1: compiler-authority references

Question: which surface of the TypeScript 7 toolchain should supply compiler-level references and definitions in batch, without a permanent dependency on an unstable API? As of 2026-09-16, `typescript@7.0.2` is the stable native compiler and it ships no supported programmatic API; the package exports `typescript/unstable/sync` and `typescript/unstable/async` clients that Microsoft labels unstable (`docs/research/2026-09-16-typescript-analysis-stack.md`). Candidates, in the order the research ranks them: a small Node indexer over the exact `typescript@7.0.2` package's `unstable/async` client that emits a Warrant-owned, versioned canonical index (TypeScript's remote object handles never cross into Rust; only the index does); the native compiler's language server over LSP for `documentSymbol`, `references`, and `definition` (stable protocol, no bulk-index request, one round trip per symbol); and the published `@sourcegraph/scip-typescript@0.4.0` artifact, which resolves `typescript@^5.6.2`, plus, as a separate instrument, a source-pinned build against TypeScript 6. A TypeScript 5 or 6 result never establishes TypeScript 7 authority, whatever it scores. Exit criteria: on the frozen corpus, for one hundred sampled exported symbols with hand-verified consumer lists, precision and recall both at or above 0.98; a full index of a repository the size of Atlas completing within the gate's instrument time limit; the index bytes identical across two runs from clean caches; request count, bytes transferred, and peak memory recorded; every unsupported construct stated explicitly; no network; a pinned, recorded instrument identity including the platform compiler package and its binary digest. Fallback if none passes: binding-level stays, and contracts that need more are labeled. The spike's report goes in `docs/research/` and its decision in section 20 (D10).

### 6.7 The Rust integration

The first Rust integration is deliberately small: units, targets, and resolved package-dependency edges from pinned `cargo metadata` inputs, including target and feature selection. It checks Warrant's crate direction and exercises the same integration contract as TypeScript. Its report names native build-system authority only for those edges, `symbol_level: none`, and unsupported Rust-internal visibility and references. Do not require a Rust `use` graph, macro expansion, or rust-analyzer to finish the Atlas experience. Those are later extensions using the research in `docs/research/2026-09-16-polyglot-integrations.md` if needed.

### 6.8 Parity qualification

`warrant instrument qualify lang-ts` is spike S6 made permanent. It runs the repository's pinned TypeScript compiler (`typescript@7.0.2`, `--noEmit --pretty false --traceResolution`, through a Node sidecar that runs and exits) once per owning tsconfig, including each referenced project of a solution-style build under its own config, over the frozen corpus and the current repository. Before that, `lang-ts` enumerates every literal module-bearing construct with oxc (ESM imports and re-exports, `import type`, `import = require`, literal `require`, literal dynamic `import()`, triple-slash type references, project references), recording importer, specifier, syntax kind, type-only flag, and byte span. A versioned adapter parses the trace, normalizes paths according to the project's `preserveSymlinks` setting while keeping both the reported and canonical path, and rejects any trace block it cannot bind to exactly one enumerated edge. `oxc_resolver` then resolves the same edges under the owning config with the importer format, condition names, extension set, package fields, and project references `lang-ts` derived. The comparison is per edge, keyed by (project, importer, span, specifier, resolution mode): a pass is zero disagreements in resolved-versus-unresolved outcome and zero disagreements in the final target after normalization; extra, missing, ambiguous, or unparsed edges are failures, never averaged away. The whole run repeats from clean caches and the canonical edge sets must match. The report is a feature-coverage matrix, and the integration's capability report is `parity-qualified` only for the features and `moduleResolution` modes the matrix covers with zero disagreement on the same platform; a repository that uses a feature outside that set is `unqualified` until the corpus is extended and qualification rerun. A project-level disagreement is never hidden inside a repository-wide percentage.

Two divergences are known at spec time and are the first rows of the matrix. `oxc_resolver` still falls back to `baseUrl/<specifier>` when no `paths` mapping matches, which TypeScript 6 removed as a lookup root; `lang-ts` disables that fallback for configurations on TypeScript 6 or later before qualification is attempted. And `oxc_resolver` exposes no named `node16` or `nodenext` mode; its declaration resolver claims the `bundler` algorithm, and the generic resolver takes condition names from the caller. `lang-ts` derives conditions per importer format and qualifies each mode separately, and `resolution_modes_qualified` in the capability report says which ones passed. The resolver's changelog shows active convergence with `tsc`, which is evidence of movement, not of parity; qualification is the only claim Warrant makes. The predecessor's `doctor compare` did this ad hoc; here it is the mechanism that decides the authority label, and its report is an instrument artifact with a digest in the receipt.

### 6.9 The query surface

`warrant query <question> [args]` answers W03's questions from the model, each row labeled with its basis (`declared`, `observed-static`, `observed-test`, `inferred`):

- `owner <path|symbol>`: the module and its declared owner and intent.
- `interface <module>`: declared interface, observed interface, and the difference.
- `consumers <symbol>`: files and modules that import the binding, with the re-export chain shown.
- `why <from-module> <to-module>`: the contract that permits or forbids the edge, and the ruling that established it.
- `blast <path|symbol>`: everything reachable from the subject through reverse edges, grouped by module, with entrypoints and external consumers marked.
- `similar <name>`: exact-name matches and structural matches (same export shape, same effect declarations) from the graph; semantic similarity only when a judgment backend is enabled, labeled `inferred`.
- `unresolved`: every unresolved edge and unsupported construct with reasons.

`warrant query --sql "<select statement>"` runs read-only SQL against the model with an authorizer that rejects anything but `SELECT`, a row cap, a byte cap, and a `truncated: true` flag with a continuation hint when either cap is hit (W18). Every agent already speaks SQL; the schema is documented in `schemas/model.md` and returned by `warrant query --schema`.

### 6.10 Considered and rejected

- An in-memory graph only (the predecessor's `petgraph` model).

  It could not be queried by agents, could not be reused across commands, and had no identity. `petgraph` is still used inside evaluation for SCC and path algorithms over data loaded from SQLite.
- Compiler-only analysis (tsc for everything).

  Correct and slow, with a Node process on every lane check; it remains the authority path for references (spike S1) and the parity oracle for resolution.
- Treating the `typescript/unstable/*` exports as a stable API, or waiting for a stable one.

  The exports are exact-pinned behind a spike and an instrument lock; a version bump is an instrument change and is classified as one.
- Using the published `scip-typescript` artifact as TypeScript 7 authority. It resolves TypeScript 5; a 5.x or 6.x index is a different instrument with a different authority label.
- tree-sitter for TypeScript. No binding resolution; it is the right lowest layer for languages without an integration and for ast-grep patterns, and the wrong one for a program model.
- Inferring registrations from function names (`register*`, `use*`). Exactly the guess the vision forbids; declarations replace inference.
- A graph database. One more server, one more query language, no agents that speak it natively.

## 7. Contracts and the policy language

Contracts are how architectural intent becomes executable. The language is small, declarative, and typed; it is not a programming language, and W05's warning that policy must not become "a second application written in a poorly supported programming language" was the constraint I kept in front of me while designing it.

### 7.1 Principles

- A contract speaks the domain's vocabulary: modules, interfaces, effects, state, capabilities. Filename globs appear only to say where a module's files live (HC15).
- Every contract carries its intent (why), its owner, its authority, its checkable claim, required evidence capabilities, enforcement mode, and limits.

  Intent can be broader than an automated check, but the verdict names the checked claim, never silently the broader intent.
- Enforcement bases have a preference order, strongest first:

  the compiler's or build system's own visibility (a Rust `pub(crate)`, a cargo dependency edge, a package `exports` map that resolution honors), the compiler-aligned program model (a resolved edge or reference), a structural pattern (an ast-grep match), and last a declaration (what the policy asserts and nothing observed). Each obligation records the basis it used and that basis's limits (section 8.1). A contract that names a weaker basis than the strongest one the enabled integrations offer says why in `limits`, and lint reports the gap, so that "the compiler refuses this" and "a pattern did not match" never read alike in a finding or in the effective-policy explanation.
- Every contract has a class:

  `invariant` (a property of the product), `preference` (a property of the code's style, still deterministic), or `migration` (temporary, with an expiry). The class decides the default consequence of a violation.
- Conflicts are configuration defects, never resolved by order. Precedence exists only as an explicit `overrides` that names the contract it overrides and carries its own authority.
- The compiled result, the effective policy, is a canonical document with a digest.

  Proven normalization rules remove irrelevant YAML differences. Other changes remain explicit and are classified under section 11.5; equal outcomes on one source tree do not establish equal meaning.

### 7.2 Files

`warrant/warrant.yaml` selects integrations, policy, profiles, and inventory settings. It proposes gate requirements but cannot override protected acceptance configuration.

```yaml
schema_version: warrant.manifest/1
integrations:
  lang-ts:
    enabled: true
    tsconfig: tsconfig.json
    framework_adapters: [nextjs-routes]
  lang-rust:
    enabled: false
policy:
  paths: ["warrant/policy/*.yaml"]
profile: warrant/profiles/trey.yaml
inventory:
  classes:
    - { class: migration, files: ["packages/core/drizzle/**/*.sql"] }
    - { class: script, files: ["scripts/**"] }
  unknown: report            # or block
  generated:
    - { files: ["packages/core/src/generated/**"], producer: "npm run codegen", reproducible: true }
  vendored:
    - { files: ["vendor/**"], source: "https://…", version: "1.4.2", treatment: excluded-from-contracts }
snapshot:
  max_file_bytes: 8388608
gate:
  required_evidence: [evidence.worker-gate, evidence.web-gate]
  trust_root_hint: "~/.config/warrant/allowed_signers"   # documentation; the gate is given the real path
instruments: warrant/instruments.lock
```

`warrant/policy/*.yaml` files contain `contracts:` and `declarations:` lists and are compiled together in a deterministic order (sorted path) that affects nothing but error message order, because the compiler forbids any semantics that depend on order.

### 7.3 Selectors

Selectors name subjects. They are typed so that lint can tell an empty selector from a misspelled one.

- `modules: ["core.actions", "web.*"]`: module ids, with a trailing `*` wildcard on dotted segments.
- `packages: ["@dbos-inc/*", "react"]`: third-party package names.
- `symbols: ["core.actions::run", "core.gmail::sendMessage"]`: module-qualified exported names.
- `units: ["apps/worker"]`: compilation or package units.
- `files: ["packages/core/src/actions/**"]`: only inside a `module` contract, and only to say where the module lives.
- `stores: ["approvals"]`: named state stores from `declarations.stores`.

A selector that matches nothing at the snapshot is a lint error (`empty-selector`) unless it is marked `may_be_empty: true` with a reason, because a rule that can never fire is either a typo or a lie about coverage (W06).

### 7.4 Contract kinds

Common fields on every contract: `id`, `kind`, `intent`, `owner`, `authority` (`ruling:<id>` or `draft`), `class` (default `invariant`), `expires` (required when `class: migration`), `claim`, `requires_capabilities`, `enforcement` (`static`, `pattern`, `evidence`, `mixed`), `limits`, `on_violation` (`fail`, `review`, `note`; defaults `fail` for invariants, `note` for preferences under R16, `fail` for migrations), `supersedes` and `overrides` (named contract ids). Short examples omit repeated common fields; the published schema requires them or supplies the explicitly documented defaults.

The claim vocabulary is closed and versioned with the evaluator. Initial claims include `declared-ownership`, `resolved-dependency-boundary`, `observed-consumer-boundary`, `recognized-write-boundary`, `required-structure`, and `tested-behavior`. Their required capabilities include `module-resolution`, `binding-references`, `recognized-call-sites`, `structural-match`, and authenticated test-case results for named scenarios. Each contract kind has a fixed mapping from claim to required capabilities; a user cannot lower that mapping by editing a field. Compiler references are a separate capability and do not imply control flow or taint flow. A contract requesting an unsupported proof is `enforcement-unsupported`, including under `enforcement: mixed` or `pattern`.

An observation-only preference with `on_violation: note` creates visible, counted advice, not an acceptance obligation. It remains in the check and verdict's advice output, including omitted counts when bounded. A human may explicitly choose `fail` or `review` for a preference; either promotion is a signed, classified policy amendment. Invariants cannot use `note`. Removing an acceptance requirement by converting it to advice is a widening change. Exceptions and accepted limitations stay visible as human dispositions, not as stronger technical proof.

`module`. Declares a module, where it lives, what it owns, and its interface.

```yaml
- id: core.actions
  kind: module
  intent: The one dispatcher. Every tool call on every transport enters actions.run; no caller outside this module knows about lock order, approval consumption, or the enqueue port.
  owner: trey
  authority: ruling:2026-09-20-bootstrap
  files: ["packages/core/src/actions/**"]
  interface:
    entry: packages/core/src/actions/index.ts
    exports: [run, plan, execute, ActionDenied, ApprovalRequired]
  owns:
    responsibilities: [tool dispatch, approval consumption]
    stores: [approvals, tool_executions]
  dynamic_loads: forbid          # forbid | declared-only | unresolved-allowed (widening)
  enforcement: static
  limits: Ownership of a responsibility is a declaration. The graph proves who imports the interface and which edges leave the module; it does not prove the responsibility is implemented here rather than elsewhere.
```

`dependency`. Permits or forbids edges between modules and packages. `default: deny` turns the `allow` list into the only permitted targets.

```yaml
- id: layering.core-independent
  kind: dependency
  intent: core imports no runtime SDK and no application package, so the worker stays swappable and the appliance stays possible.
  owner: trey
  authority: ruling:2026-09-20-bootstrap
  from: { modules: ["core.*"] }
  deny:
    packages: ["@dbos-inc/*", "next", "react"]
    modules: ["web.*", "worker.*", "cli"]
  include_type_only: false
  declares: []                    # declared edges for dynamic loads, each with a reason and a ruling
  enforcement: static
  limits: Binding-level. A dependency reached only through reflection or a non-literal dynamic import is reported as unresolved, not as permitted.
```

`interface`. Constrains how a module is consumed: through its entry file only, and by whom.

```yaml
- id: core.actions.consumers
  kind: interface
  intent: Transports consume the dispatcher only through its public entry; no deep imports of lock or ledger internals.
  module: core.actions
  only_via_entry: true
  consumers: { modules: ["web.*", "worker.*", "cli"] }
  enforcement: static
  limits: Binding-level consumers. A consumer that reaches the module through a type-derived reference is not observed.
```

`effect`. Declares an effect interface, who may call it, what context the call must carry, and what evidence controls it.

```yaml
- id: effect.email-send
  kind: effect
  intent: Centralize email sending through the approved adapter.
  claim: observed-consumer-boundary
  requires_capabilities: [binding-references, recognized-call-sites, structural-match]
  owner: trey
  authority: ruling:2026-09-20-bootstrap
  interface: { symbols: ["core.gmail::sendMessage"] }
  permitted_callers: { modules: ["worker.workflows.send"] }
  required_context:
    pattern:
      id: effect.email-send.ctx
      rule: { pattern: "sendMessage($CTX, $$$ARGS)", constraints: { CTX: { regex: "^ctx$" } } }
  evidence:
    - { kind: test-receipt, tag: "effect-control:email-send", evidence_kind: integration }
  enforcement: mixed
  limits: Restricts observed references to this adapter and recognizes a first argument named ctx. It does not prove valid authorization, a live transaction, or that all email routes use this adapter. The named integration evidence establishes only the scenarios its authenticated report identifies.
```

`state`. Declares who may write a named store and how write sites are recognized.

```yaml
- id: state.approvals
  kind: state
  intent: Keep approval writes in the dispatcher.
  claim: recognized-write-boundary
  requires_capabilities: [structural-match]
  store: approvals
  writers: { modules: ["core.actions"] }
  write_sites:
    pattern: { rule: { pattern: "db.insert(approvals)" } }
  enforcement: pattern
  limits: Restricts only writes recognized by this ORM pattern. It cannot establish exclusive ownership of all writes; raw SQL and dynamic table access are outside the checked claim. A requirement for all writes needs separately qualified evidence, not this pattern under a stronger name.
```

`capability`. Links the parts of a user-visible feature; each registered instance must satisfy every link.

```yaml
- id: capability.undoable-operation
  kind: capability
  intent: Undoable operations use the existing compensation owner and have evidence for authorization denial, readback, and failure behavior.
  claim: required-structure
  requires_capabilities: [binding-references, structural-match]
  owner: trey
  authority: ruling:2026-09-20-bootstrap
  instances: { registry: undo-registry }
  requires:
    - { link: entrypoint, basis: registry }
    - { link: authorization-call-occurrence, pattern: { rule: { pattern: "plan($$$)" }, within: instance } }
    - { link: compensation-owner, one_of_module: core.actions.undo }
    - { link: authorization-denial-test, evidence: { kind: test-receipt, tag: "undo-denied:{instance}", evidence_kind: integration, scenario: denied-request-produces-no-effect } }
    - { link: readback-test, evidence: { kind: test-receipt, tag: "undo:{instance}", evidence_kind: integration, scenario: undo-restores-recorded-state } }
    - { link: failure-path-test, evidence: { kind: test-receipt, tag: "undo-failure:{instance}", evidence_kind: integration, scenario: partial-failure-preserves-consistency } }
  enforcement: mixed
  limits: Structural links and passing named behavior tests are reported separately. A plan call does not establish guarding control flow; linked parts do not prove a working end-to-end capability. Test reports must identify the required executed cases and outcomes. They are sampled behavior evidence, not universal proof.
```

`pattern`. An ast-grep rule with Warrant metadata, forbidden or required within a scope.

```yaml
- id: style.no-default-exports
  kind: pattern
  class: preference
  intent: Named exports only, so that the interface of a module is a list of names.
  scope: { modules: ["core.*"] }
  rule: { pattern: "export default $X" }
  forbid: true
  enforcement: pattern
  limits: Syntactic. A default export produced by a build transform is not seen.
```

`evidence`. A standing requirement that a named receipt exists for this snapshot.

```yaml
- id: evidence.worker-gate
  kind: evidence
  intent: The worker's integration suite ran against a real database on the exact candidate.
  requires: { kind: test-receipt, tag: worker-suite, evidence_kind: integration, unit: apps/worker }
  enforcement: evidence
  limits: A receipt proves the command ran on this snapshot and what it reported. It does not prove the suite is adequate.
```

`data`. A sensitive-data declaration checked at binding level, with its limits stated loudly.

v1 has no declassification or `removes_sensitivity` operation. Summarizing data does not remove its sensitivity by declaration; stronger data-flow or declassification claims are unsupported.

```yaml
- id: data.private-mail
  kind: data
  intent: Restrict observed imports of private-mail source interfaces.
  claim: observed-consumer-boundary
  requires_capabilities: [binding-references]
  sources: { symbols: ["core.gmail::fetchThread", "core.gmail::fetchMessage"] }
  permitted_sinks: { modules: ["core.model", "core.memory"] }
  enforcement: static
  limits: v1 checks that the source symbols are consumed only by permitted modules at binding level and that no other module imports them. It does not follow values through variables or calls. It is not a taint analysis and must not be described as one.
```

### 7.5 Declarations

Declarations are policy inputs that are not contracts: they tell Warrant how to see things it cannot infer.

```yaml
declarations:
  stores:
    - { id: approvals, kind: table, defined_in: "packages/core/src/schema/approvals.ts" }
  registries:
    - id: undo-registry
      intent: Undoable operations are registered by calls to registerUndo in the undo module.
      file: packages/core/src/actions/undo/registry.ts
      rule: { pattern: "registerUndo($NAME, $HANDLER, $COMPENSATION)" }
      captures: { name: NAME, symbol: HANDLER, compensation: COMPENSATION }
      entrypoint_kind: registry
    - id: mcp-tools
      file: packages/core/src/tools/registry.ts
      rule: { pattern: "registerTool($DEF)" }
      captures: { symbol: DEF }
      entrypoint_kind: registry
  external_consumers:
    - { id: claude-connector, depends_on: { symbols: ["core.tools::*"] }, compatibility: "tool names and outputSchema are stable within a major" }
  loaders:
    - { id: migrations, file: "packages/core/drizzle.config.ts", references: { files: ["packages/core/drizzle/**"] } }
```

Everything a declaration says is `basis: declared` in the model. A registry declaration's matches are entrypoints; a store declaration makes `state` contracts possible; an external consumer makes a public interface change a finding even when no in-repository consumer exists (W08). Declarations are part of the effective policy and their changes are classified like any other policy change: removing a registry declaration is a widening, because it removes entrypoints from the denominator.

### 7.6 Effective policy

`warrant policy compile` loads the manifest's policy files, validates versions and fields, checks unique ids and rule conflicts, expands explicit defaults, and canonicalizes the rule definitions. Its `policy_digest` includes selectors as expressions, declarations, consequence settings, profile requirements, and the admitted instrument/configuration semantics. It excludes file placement, incidental formatting, signature bytes, and authority-pointer bookkeeping; raw source blobs and authority references are recorded separately. An unchanged rule has the same policy digest when a new source file begins matching it. Canonicalization normalizes only transformations proven equivalent for the supported grammar, not arbitrary logically equivalent programs.

Applying that policy to a snapshot resolves selectors against the inventory and model, checks snapshot-specific consistency, derives concrete obligations, and computes a separate `obligations_digest`. Receipts bind both digests. A human signs the canonical rules and admitted configuration, not a changing list of today's files. The signature chain authorizes those rules; changing `authority: draft` to a verified ruling reference does not alter their semantic digest or create a self-referential signature.

Conflict classes, each a lint error:

- Two `module` contracts whose `files` selectors overlap.
- Two `dependency` contracts where one allows and the other denies the same edge, without an `overrides` between them.
- Two `state` contracts naming the same store with different writers.
- An `interface` that lists exports the module does not export.
- A `capability` whose registry has no instances (unless `may_be_empty`).
- A claim whose required capabilities are unsupported, under any enforcement mode:

  `enforcement-unsupported` with the missing capability named. The author may propose a genuinely narrower claim or add qualified evidence. A limits paragraph alone does not resolve the mismatch; any reduction of an approved requirement needs a signed amendment.
- A `migration` without `expires`, or an expired migration still present (reported; the contract stops applying at expiry and the gate reports it as `expired-contract`).
- An `overrides` without `authority: ruling:`.

Drift checks (W09), reported by `warrant policy lint` as findings of class `drift`, never silently: modules with no files; declared interfaces with no consumers anywhere, including declared external consumers (reported, not blocking, because a new interface has none yet); registries with no matches; stores with no recognized write sites; capabilities whose instances all fail the same link (which usually means the pattern is wrong, not the code); declared external consumers whose symbols no longer exist.

Unratified contracts can be evaluated in advisory checks to preview any proposed policy. Authoritative acceptance uses the protected approved-policy anchor and a verified amendment chain. Any candidate request to replace that policy, including a narrowing, requires a human signature; classification changes the explanation, not the authority requirement. A separate census proposal not selected as active policy remains a proposal and does not itself request adoption. Ordinary source edits under identical approved rules require no new ruling. This preserves experimentation without letting an agent install new obligations for everyone (W15).

### 7.7 Explaining the effective policy

`warrant policy effective <path|symbol|module>` prints every contract that applies to the subject, the selector that matched it, its authority and class, and, for dependency questions, the specific allow or deny entry that governs, with the ruling that established it. `warrant policy diff` is section 11.5.

### 7.8 A short worked policy

For a repository shaped like Atlas, the whole architecture in section 1.5 of that product's spec compiles to roughly: one `module` contract per bounded context and per deployable; one `dependency` contract for `core`'s independence and one for the transports; one `interface` contract for the dispatcher; three `effect` contracts (send, delete, external model call); one `state` contract per ledger table; two `capability` contracts (tool, undoable operation); two `evidence` contracts (the gates); a handful of `pattern` preferences; and the declarations for the three registries and the ledger stores. The census (section 15) proposes the modules and dependency contracts from observation; the effects, capabilities, and evidence requirements are written by hand, because they encode intent the code cannot show.

### 7.9 Considered and rejected

- Rego and OPA.

  A general policy language with its own evaluation semantics; agents would have to learn it, and its verdicts are opaque to a human reading YAML. Warrant's policy is data, and its semantics are the compiler's, tested by fixtures.
- Cedar.

  An excellent permission language with formal analysis, but its model (principal, action, resource) does not express graph obligations like "every registered instance links to a compensation owner." It remains the leading candidate for a future permission-widening analysis over compiled `dependency` contracts.
- A TypeScript or Rust DSL for policy.

  Expressive, and exactly the wrong thing: policy that is code is harder to sign, to diff semantically, and to keep out of the agent's pen.
- Built-in layer taxonomies ("domain, application, infrastructure"). W's "not a rigid taxonomy" objection stands; layers are one shape of `dependency` contract, not a primitive.
- Treating missing required analysis as a warning. Advice-only preferences may print without blocking, but required analysis and evidence cannot silently become advice.

## 8. Evaluation and findings

### 8.1 Obligations

Evaluation begins by deriving obligations: one contract applied to one subject at one snapshot yields one checkable requirement with a stable id. Obligation types, each named by a `contract.kind` and a check:

- `edge-forbidden` and `edge-not-allowed` (`dependency`): an observed edge from the scoped module to a denied target, or, under `default: deny`, to a target outside the allow list.
- `interface-bypass` (`interface`): a consumer imports a file of the module other than its entry.
- `consumer-not-permitted` (`interface`): a consumer outside the `consumers` selector imports the interface.
- `caller-not-permitted` (`effect`): a file outside `permitted_callers` references the effect interface.
- `context-missing` (`effect`): a call site does not match `required_context`.
- `writer-not-permitted` (`state`): a recognized write site is outside `writers`.
- `link-missing` (`capability`): an instance lacks a required link.
- `pattern-forbidden` and `pattern-required` (`pattern`).
- `evidence-required` (`evidence`, and the `evidence` entries of `effect` and `capability`).
- `sink-not-permitted` (`data`).
- `inventory-owned`, `inventory-readable`, `resolution-complete` (completeness obligations the inventory and model produce on their own).
- `cycle-forbidden` (a `dependency` contract with `cycles: forbid` over its scope).
- `declared-consistency` (drift obligations from section 7.6).

Every obligation records its typed claim, required capabilities, actual evidence basis (`visibility`, `observed-static`, `pattern`, `declared`, or `observed-test`), and limits. `observed-test` also names authenticated case outcomes and the scenario they exercise. The explanation cannot promote one basis to another. A declaration or occurrence match remains such even when approved by a human. Advice-only preferences are not acceptance obligations and cannot satisfy one.

### 8.2 Evaluation outcomes

Each obligation resolves to exactly one of: `satisfied` (with the evidence reference that satisfied it, which may be a graph observation, a pattern match, an external receipt, or a valid exception ruling); `unsatisfied` (a finding); or `undeterminable` (the model lacks what the check needs: an unread file in scope, an unresolved edge, an unsupported construct, a missing instrument). `undeterminable` is neither pass nor fail. It feeds the analysis facet, and HC3 says it can never decay into `satisfied`.

### 8.3 Findings

A finding is an unsatisfied obligation plus the explanation W19 asks for. The seven explanation fields are required; a field that genuinely has no answer is `null` with a `null_reason`.

```json
{
  "schema_version": "warrant.finding/1",
  "id": "f_7q3k9m2xw4pbz1a",
  "contract": "capability.undoable-operation",
  "obligation": "link-missing",
  "subject": { "kind": "capability-instance", "id": "undo-registry:group_debrief" },
  "consequence": "fail",
  "basis": "resolved-registration-graph",
  "locations": [ { "path": "packages/core/src/actions/undo/registry.ts", "line": 88, "column": 3 } ],
  "explanation": {
    "unsatisfied": "Instance group_debrief registers a compensation handler in worker/workflows/debrief, but the contract requires the compensation owner to be a symbol in core.actions.undo.",
    "established_by": "registry undo-registry matched registerUndo at registry.ts:88; the third capture resolves to worker/workflows/debrief.ts#compensateGroup.",
    "why": "Contract capability.undoable-operation, ruled in ruling:2026-09-20-bootstrap by trey: one compensation owner per undoable operation.",
    "path": ["registry.ts:88 registerUndo", "→ worker/workflows/debrief.ts#compensateGroup (compensation)"],
    "existing_interface": "core.actions.undo exports registerCompensation(name, handler); three instances use it.",
    "focused_check": "warrant check --contract capability.undoable-operation --instance group_debrief",
    "uncertainty": "If compensation for grouped operations must differ in kind, the right fix is an amendment to the contract, not a second owner."
  },
  "remediation": {
    "kind": "code",
    "alternatives": [
      { "kind": "code", "summary": "Register the compensation through core.actions.undo::registerCompensation." },
      { "kind": "amendment", "summary": "Propose a ruling that grouped operations may own their compensation; this widens capability.undoable-operation and requires a signature." }
    ]
  },
  "group": { "cause": "c_2m8n...", "role": "primary", "symptoms": 0 },
  "exception": null
}
```

`consequence` is the contract's `on_violation` (`fail`, `review`, `note`), and it is what the verdict's facets read; a finding never carries a severity. `remediation.kind` distinguishes changing the code from changing the policy; an amendment is a ruling draft, never an autofix (W19).

### 8.4 Grouping by cause

Findings that share a cause are grouped: one primary, the rest as symptoms with a count and a `warrant explain --group <cause>` path to list them. Causes are computed, not guessed: a missing module assignment groups every edge finding from its files; a registry pattern that matches nothing groups every `link-missing` on its instances; an `interface-bypass` from one entry-file rename groups every consumer. A single wrong registration produces one primary finding, not fifty (W19).

### 8.5 Signals

Signals are reviewable indicators that are neither findings nor facets. They are computed by comparing the candidate to its base, they carry numbers and locations, and they never carry a score (HC9). The profile decides whether a signal is `note` or `review` (which sets `approval: required`). An advisory check may select its source base. When a signal can affect authoritative acceptance, the gate's protected configuration supplies the comparison base; the candidate cannot choose itself as the base to hide a change. A required comparison without its base is incomplete, not zero change.

Metric-gaming signals (W13) include widened exclusions, reclassified source or tests, and removed tests from required units. Language-specific patterns count changed assertions, suppressions, and type escapes. Where an external complexity instrument supports the comparison, a signal can show complexity moved between functions rather than removed. Each is an observation to interpret, not proof of intent to game a metric.

Conceptual-expansion signals (W11): new public symbols per module; new modules; new declared stores (state owners); new module-to-module edges; new cycles; new configuration knobs (declared patterns for environment reads and config keys); changes that touch more modules than the capability they claim to complete (change locality, computed from the capability instances the diff touches); interfaces whose consumers now need to know more (new required parameters on exported functions, counted syntactically).

Every signal is presented with the counterexample that could justify it, in the profile's words, so that a reviewer can reject a cleanup that improves numbers and hurts the design (story E) and accept one that deletes tests because the tested thing no longer exists.

### 8.6 Stable identity

`finding.id` is the first sixteen characters of a base32 sha256 over the contract id, the obligation type, the subject identity, and the basis kind. Subject identity is structural: a module id, a module-to-module edge, a module-qualified symbol, a capability instance name, or a store name. Line numbers never enter the identity. Between a base and a candidate, git rename detection maps old paths to new ones for file-based subjects, and a symbol keeps its identity when its export name and module are unchanged; a rename that git cannot attribute with confidence produces a new id with `renamed_from: uncertain`, which is honest and which W16 asks for ("a renamed file should carry an exception only if its identity and meaning are established").

### 8.7 Considered and rejected

- Severity levels on findings. Replaced by `consequence`, which is the contract author's decision about what happens, not a label about how bad it looks.
- Autofix patches.

  W20 allows small inspectable patches bound to a snapshot; v1 emits suggestions and the focused check, and a patch-producing mode is deferred until findings are stable enough to bind a patch to.
- Positional fingerprints as the primary identity.

  The predecessor kept both a stable and a positional fingerprint; the positional one is what made a moved line look like a new finding. Position is location, not identity.

## 9. Evidence and instruments

Warrant relates evidence to obligations; it does not manufacture evidence. This section defines what counts, how it is bound to a snapshot, and how the tools that produce it are pinned.

### 9.1 Evidence kinds

- `graph`: a static observation from the program model (an edge, a symbol, an entrypoint). Basis `visibility`, `observed-static`, or `declared` (section 7.1).
- `pattern`: an ast-grep match or non-match within a scope.
- `test-receipt`: a record produced by `warrant attest run` that a command ran on an exact snapshot and what it reported.
- `report`:

  an instrument's output (SARIF or a known JSON reporter) bound to a snapshot by the receipt that produced it, because no instrument's format carries a tree id of its own.
- `observation`: a receipt that something was observed on a named deployed revision, produced by a runner that signs it.
- `exception`: a valid exception ruling (section 11.4).
- `judgment`: typed model advice, never evidence satisfying a compliance obligation. The initial product attaches it only as advice (HC1, section 16).

Test evidence describes its environment: `local` (isolated local logic), `mocked` (mocked dependencies), `integration` (named real services), or `deployed` (a named deployed revision). These are not an automatic strength ladder: a deployed UI observation does not replace a database race test. Each obligation names acceptable classes, scenarios, units, and service requirements. Producer trust is independent of class; all classes need an admitted authenticated producer before the gate credits them (HC19).

### 9.2 The attest wrapper

`warrant attest run --tag <tag> --class <class> [--unit <unit>] [--snapshot worktree|index|<rev>] -- <command...>` is a recording wrapper for existing tools. A local invocation is advisory. The same operation under an approved runner controller can produce gate-eligible evidence only when the execution contract below is met.

```json
{
  "schema_version": "warrant.test-receipt/1",
  "id": "tr_9v2...",
  "snapshot": { "repo": "sha1:…", "kind": "worktree", "tree": "sha1:71ab…" },
  "tag": "worker-suite",
  "class": "integration",
  "unit": "apps/worker",
  "command": ["npm", "run", "test:worker"],
  "cwd": "apps/worker",
  "env_recorded": { "NODE_ENV": "test", "CI": "1" },
  "env_names_present": ["DATABASE_URL"],
  "execution": {
    "source_materialization_digest": "sha256:...",
    "dependency_artifacts_digest": "sha256:...",
    "generated_inputs_digest": "sha256:...",
    "runner_profile_digest": "sha256:...",
    "services": [{ "id": "test-database", "identity_digest": "sha256:...", "state_recipe_digest": "sha256:..." }],
    "producer": null,
    "provenance": "agent-local-advisory"
  },
  "exit_code": 0,
  "stdout_digest": "sha256:…",
  "stderr_digest": "sha256:…",
  "report_files": [ { "path": "reports/vitest.json", "digest": "sha256:…", "adapter": "vitest-json@0.1.0" } ],
  "duration_ms": 41230,
  "instruments": [ { "name": "vitest", "version": "5.0.1", "source": "package-lock", "integrity": "sha512-…" } ],
  "started_at": "2026-09-16T21:10:02Z",
  "finished_at": "2026-09-16T21:10:43Z",
  "signature": null
}
```

The execution contract is checked by the protected runner controller, not asserted by candidate flags:

1. Materialize exactly the selected source in an isolated execution root and run with that root as the working directory. Index or commit selection must not execute against unrelated dirty worktree bytes. The runner constrains candidate reads to that source and the declared dependency, generated-input, and service inputs. Existing isolation provides this boundary; Warrant does not implement a sandbox.
2. Bind the dependency artifacts actually installed, generated inputs, fixtures, migrations, command/configuration identity, environment profile, and service identities or setup recipes. A lockfile alone does not identify installed bytes. An authenticated immutable image or materialization record may bind several input classes together; it names the covered content and derivations rather than omitting those identities. A `--class integration` label or an environment-variable name is not evidence that a real database was used.
3. Keep captured source immutable to the tested process, or use an approved runner mechanism that detects source mutation during execution, including a change restored before exit. Build outputs go to separate writable locations and are digested when they feed analysis or tests. Comparing the source before and after alone is insufficient.
4. Capture fresh reports from that execution, bind their digests and actual case outcomes, and reject missing, malformed, truncated, skipped, or zero-case results where the obligation requires executed passing cases. The gate admits only reports whose producer signature and runner profile match its protected configuration. Candidate code must not possess the producer credential.

An unmet execution requirement leaves a receipt advisory or unusable for the named obligation, with the missing binding stated. Source drift is `snapshot_unstable` and satisfies nothing. The examples' `signature: null` and producer `null` are deliberately local, not gate proof.

Environment values are recorded only for an approved nonsecret allowlist; secret values, secret hashes, and raw credentials never enter receipts. Service identity is a safe runner-assigned identifier, not a connection URL. Captured stdout, stderr, and native reports can contain secrets or source: keep raw artifacts in the runner's access-controlled store, and export only authorized redacted views and digests. Warrant never automatically publishes them to the public repository.

An authoritative evidence obligation requires matching source identity, tag, allowed environment class, unit, required executed cases, dependency and service bindings, instrument identities, and an authenticated admitted producer. No receipt field is accepted merely because the agent wrote it. A different tree is `stale`; an untrusted or incomplete producer binding is `missing` with its reason. v1 does not carry execution evidence between different source trees (section 13.3).

### 9.3 Instrument reports

The report formats below are researched candidates, not a mandatory adapter checklist. The first adopter's approved evidence requirements select the initial readers. Prefer a shared qualified format/library when it reduces work. Keep required Fallow, test, type, coverage, and mutation evidence; do not build extra Vitest/Jest or other readers merely to claim broad support. An unsupported required format stays missing evidence until a reader is qualified, never a reason to drop the requirement.

None of knip, Fallow, dependency-cruiser, Stryker, vitest, or jest puts a git tree id in its ordinary output, so a report cannot bind itself to a snapshot and `--snapshot` on a later import cannot prove which tree produced a file. Binding therefore comes from one of two places: the `warrant attest run` receipt that ran the instrument and digested its report (`report_files`, section 9.2), or, for reports produced elsewhere, a signed external attestation (an in-toto Statement from the CI runner) whose subject is the tree and whose predicate carries the report digest. `warrant evidence import --instrument <name> --receipt <id> <file>` and `--attestation <statement>` are the two forms; a report imported with neither is `report-unbound` and satisfies nothing (W24).

SARIF 2.1.0 is the preferred interchange wherever an instrument emits it, and the instrument's native report is always what is preserved and digested, because a normalization that drops a native field drops evidence. The JSON outputs of knip, Fallow, ESLint, oxlint, vitest, jest, and Stryker, and the text of TypeScript's diagnostics (`--pretty false`) and `--traceResolution`, are mapped by small adapters that are themselves versioned instruments; no formal JSON Schema was verified for knip, vitest, or jest, so each adapter's fixture set is the qualification contract for its format, and a format change shows up as a fixture failure before it shows up as a wrong verdict. Import records the raw file's digest (what gets signed and compared), the instrument's identity from the lock, the adapter version, and the tree id, and stores each result as a canonical evidence row keyed by instrument, rule id, and location; raw digest and canonical rows are kept apart so that a re-normalization never changes what was attested. An adapter decides what an instrument found from its parsed report, never from the exit status alone. The exit is recorded as execution evidence (the process finished, timed out, or was killed) and the finding set comes from the structured output: an instrument that exits zero with findings in its report yields those findings; an instrument that exits zero with an empty or unparseable report is `report-truncated`, never a pass; and an instrument whose exit code means "findings" (Fallow exits 1 on a successful run that found something) is read by its declared exit contract rather than treated as a crash. Kazi's parsed-result gating is the prior art for this rule (`docs/research/2026-09-16-prior-art.md`, section 8), and an adversarial fixture keeps it true here (section 17.2). A report that is truncated or fails to parse is `report-truncated` and sets the analysis facet incomplete when a contract requires that instrument.

Contracts reference instrument evidence through `evidence` requirements that describe the absence or presence of results in a scope, for example `{ kind: report, instrument: knip, rule: "unused-export", absent: true, scope: { modules: ["core.*"] } }` or `{ kind: report, instrument: stryker, metric: "mutationScore", at_least: 0.8, unit: "apps/worker" }`. A metric such as `mutationScore` is a derived value: the Stryker report schema carries mutant statuses, not the scalar, and the adapter computes it under a pinned formula (`mutation-testing-metrics@3.8.4`) that is part of the adapter's identity. Meaning is not normalized across instruments (W12): a knip result and a Fallow result about the same symbol are two evidence rows that corroborate, not one deduplicated row, and the explanation shows both.

### 9.4 The instruments lock

`warrant/instruments.lock` pins every external tool Warrant runs or ingests:

The following is an illustrative catalog. A repository's actual lock includes the instruments it uses; it is not an instruction to install every entry. The protected gate compares that lock with the admitted configuration rather than trusting candidate edits.

```yaml
schema_version: warrant.instruments/1
instruments:
  - name: typescript
    kind: npm
    version: "7.0.2"
    integrity: "sha512-…"              # registry integrity of the package
    lock_digest: "sha256:…"            # digest of the resolved dependency lock that installed it
    platform_package: "@typescript/native-preview-linux-x64"   # illustrative; the platform package typescript@7.0.2 selected
    platform_binary_digest: "sha256:…"
    roles: [parity-oracle, diagnostics]
    adapters: { traceResolution-text: "0.1.0", diagnostics-text: "0.1.0" }
    required: true
    timeout_s: 600
  - { name: knip, kind: npm, version: "6.36.0", integrity: "sha512-…", role: report, adapter: "knip-json@0.1.0", invocation: ["npx", "knip", "--reporter", "json"], required: false }
  - { name: fallow, kind: npm, version: "3.27.0", integrity: "sha512-…", role: report, adapter: "fallow-json@0.1.0", findings_exit_codes: [1], required: false }
  - { name: stryker, kind: npm, package: "@stryker-mutator/core", version: "10.0.0", role: report, adapter: "stryker-json@0.1.0", schema: "mutation-testing-report-schema@3.8.4", metrics: "mutation-testing-metrics@3.8.4", required: false }
  - { name: vitest, kind: npm, version: "5.0.1", role: test-runner, adapter: "vitest-json@0.1.0" }
  - { name: jest, kind: npm, version: "30.5.1", role: test-runner, adapter: "jest-json@0.1.0" }
  - { name: dependency-cruiser, kind: npm, version: "18.3.1", role: census-importer }
  - { name: tree-sitter-typescript, kind: grammar, version: "0.23.2", role: pattern-grammar }
  - { name: ast-grep-rules, kind: embedded, version: "warrant-0.1.0" }
  - { name: lang-ts, kind: integration, version: "0.1.0", qualified_on: "corpus:2026-09-16", modes_qualified: [bundler] }
  - { name: nextjs-routes, kind: adapter, version: "0.1.0" }
```

An npm entry records the registry integrity, the digest of the resolved dependency lock that installed it, the runtime version actually observed at run time, the platform binary digest where the package selects one, the output schema identity where one exists, and the adapter version; the receipt copies whichever of these the run touched. Two installs with the same `version` and different `lock_digest` are different instruments, because a transitive dependency can change what a report says.

`warrant instrument status` compares the lock to what is installed and what authenticated receipts report. A required instrument that is missing or mismatched sets `analysis: incomplete` (HC8). There is no downgrade flag; restore the approved instrument or propose a signed change. Installation remains the job of the existing package or release manager, not a Warrant package-management command.

### 9.5 Instrument upgrades

`warrant instrument upgrade <name> <version>` runs the frozen corpus (section 17.3) under the old and the new identity and writes a before-and-after report to `docs/instruments/<date>-<name>-<old>-<new>.md`: findings added, removed, and changed by contract; model digest changes per corpus repository; parity changes for resolvers; wall-clock and memory. The lock is not changed by this command. Adopting the new version is a change to the lock, which the policy-diff classifier treats as `uncertain`, which is widening (HC6), which requires a ruling whose rendered effect summary is that report. A tool getting more permissive shows up as an instrument change, never as an improvement in the code (W31, story F). The same procedure applies to Warrant's own version: `warrant self-qualify` compares corpus output between the installed build and a candidate build.

### 9.6 Considered and rejected

- Running tests inside Warrant. It is not a test runner (W12); the attest wrapper records, it does not execute test logic.
- Accepting "tests passed" as a boolean from CI. Unbound to a tree and to a class, it is exactly the laundering W24 describes.
- Normalizing instrument findings into one taxonomy. Location and provenance normalize; meaning does not, and pretending otherwise hides disagreement between instruments.
- Auto-updating instruments. A quiet upgrade is a quiet policy change.
- Exit status as the finding set.

  A zero exit says the process finished; the report says what it found. An adapter that reads the exit alone turns a report full of findings into a pass, and a truncated report into a pass.

## 10. The verdict and the receipt

### 10.1 Four facets

A verdict carries four facts, each computed independently, none able to override another:

- `compliance`: `pass` or `fail`. `fail` when at least one finding has `consequence: fail` and no valid exception.
- `analysis`: `complete` or `incomplete`, with the list of reasons from section 5.5 and section 8.2.
- `evidence`: `satisfied`, `missing`, or `stale`, with the list of required receipts and their status.
- `approval`: `none`, `required`, or `granted`, with the list of items that require a ruling:

  all proposed changes to approved rules, findings with `consequence: review`, signals deliberately routed to review, instrument changes, and expired rulings still relied on. Optional model advice creates no approval item in the initial product. Any later model-triggered required review explicitly blocks acceptance (D3).

`acceptance` is derived and never stored as an input: `accepted` when compliance is `pass`, analysis is `complete`, evidence is `satisfied`, and approval is `none` or `granted`; otherwise `blocked`, with the responsible facets named. A run can be `compliance: fail` and `analysis: incomplete` at the same time, and the verdict shows both (W17).

Advisory checks (`warrant check`) produce `mode: check`, acceptance `check-clean` or `check-blocked`, and a predicate type distinct from gate receipts. A narrowed check also records its selected scope and omitted obligations. Hook diagnostics use `mode: diagnostic` and have no acceptance field. None can authorize acceptance or satisfy the requirement for a full gate (W17).

### 10.2 Exit codes

- 0: `accepted` (gate) or `check-clean` (advisory check). Non-verdict query commands use their own successful-result semantics.
- 1: `blocked` or `check-blocked`, for any facet. The JSON says which.
- 2: the run could not evaluate: invalid invocation, policy compile error, snapshot unbuildable, no trust root given to a gate.
- 3: internal failure or an instrument crash that is not a policy matter.
- 130 and 143: cancelled by signal; no artifact written.

One code for "not accepted" is deliberate. Agents read the JSON; shell scripts and CI need one branch; and distinguishing "blocked by findings" from "blocked by missing evidence" in the exit code would invite a script to treat one of them as softer. The facets are in the document; the exit code says whether the merge may proceed.

### 10.3 The verdict document

The candidate below has complete analysis but is blocked on a contract, required evidence, and a rule change. The satisfied worker receipt represents authenticated gate evidence, not the unsigned local example in section 9.2.

```json
{
  "schema_version": "warrant.verdict/1",
  "mode": "gate",
  "snapshot": { "…": "section 4.5 manifest" },
  "policy": { "digest": "sha256:…", "obligations_digest": "sha256:…", "approved_anchor_digest": "sha256:…", "files": [ { "path": "warrant/policy/core.yaml", "blob": "sha1:…" } ], "unratified": [] },
  "profile": { "path": "warrant/profiles/trey.yaml", "digest": "sha256:…" },
  "inventory": { "digest": "sha256:…", "summary": { "…": "section 5.5" } },
  "model": { "digest": "sha256:…", "capabilities": [ { "…": "section 6.1" } ] },
  "instruments": [ { "name": "typescript", "version": "7.0.2", "matched_lock": true } ],
  "compliance": { "status": "fail", "failing": ["f_7q3k9m2xw4pbz1a"] },
  "analysis": { "status": "complete", "reasons": [] },
  "evidence": { "status": "missing", "required": [ { "contract": "evidence.worker-gate", "status": "satisfied", "receipt": "tr_gate_worker…" }, { "contract": "capability.undoable-operation#undo:group_debrief", "status": "missing" } ] },
  "approval": { "status": "required", "items": [ { "kind": "policy-widening", "change": "dependency layering.core-independent: deny list shrank by @dbos-inc/*", "class": "widening", "ruling": null } ] },
  "acceptance": "blocked",
  "blocked_by": ["compliance", "evidence", "approval"],
  "findings": [ { "…": "section 8.3" } ],
  "signals": [ { "kind": "suppressions-added", "count": 3, "locations": ["…"], "consequence": "review" } ],
  "judgments": [],
  "denominator": { "files": 1834, "modules": 23, "contracts": 31, "obligations": 4120, "evaluated": 4120, "undeterminable": 0 },
  "evaluated_at": "2026-09-16T21:11:09Z",
  "clock_source": "system",
  "tool": { "name": "warrant", "version": "0.1.0", "git_sha": "…", "target": "x86_64-unknown-linux-gnu", "binary_digest": "sha256:…" },
  "receipt": "rc_…"
}
```

`denominator` is HC17: the document always says how much was evaluated and how much could not be.

### 10.4 The receipt

A receipt is an in-toto Attestation Statement v1 whose subject is the snapshot tree and whose predicate is the verdict's inputs and results:

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [ { "name": "snapshot", "digest": { "gitTree": "71ab…", "sha256": "<inventory digest>" }, "annotations": { "identity": "commit", "object_format": "sha1" } } ],
  "predicateType": "https://warrant.dev/attestation/receipt/v1",
  "predicate": {
    "mode": "gate",
    "producer": { "principal": "gate@ci", "signed": true },
    "snapshot": { "…": "manifest" },
    "policy": { "digest": "…", "obligations_digest": "…", "approved_anchor_digest": "…", "files": [ "…" ] },
    "inventory": { "digest": "…", "summary": { "…": "…" } },
    "model": { "digest": "…", "capabilities": [ "…" ] },
    "instruments": [ "…as ran, with digests…" ],
    "evidence": { "receipts": [ { "id": "tr_…", "digest": "sha256:…", "tree": "sha1:…" } ], "reports": [ { "instrument": "knip", "digest": "sha256:…" } ], "exceptions": [ { "ruling": "r_2026-09-18-clone-3", "signature_verified": true, "principal": "trey", "valid_at_clock": true } ] },
    "judgments": { "profile_digest": "…", "backend": "typesafe", "status": "obtained", "entries": [ { "question_digest": "…", "state_digest": "…", "response_model": "jev-1.12", "answer": { "type": "noul", "noul": "0.81" }, "request_id": "…", "usage": { "input_tokens": 2140, "output_tokens": 9 }, "state_truncated": false } ] },
    "verdict": { "compliance": "fail", "analysis": "complete", "evidence": "missing", "approval": "required", "acceptance": "blocked", "finding_ids": [ "…" ], "signal_digest": "…" },
    "trust_root": { "digest": "sha256:…", "principals": ["trey"] },
    "evaluated_at": "…", "clock_source": "system",
    "tool": { "name": "warrant", "version": "…", "binary_digest": "sha256:…" },
    "execution_profile_digest": "sha256:...",
    "uncaptured_inputs": []
  }
}
```

The receipt is written as RFC 8785 canonical JSON, UTF-8, no trailing newline, and the receipt id is the sha256 of those bytes. Verification compares bytes and never re-serializes; a receipt that has been reformatted fails at step 1 below. Signed records carry no JSON floats: counts are integers and every other number, including a judgment probability, is a decimal string exactly as the backend returned it, because RFC 8785 number serialization is the part of the standard implementations most often get wrong (`docs/research/2026-09-16-signing-and-attestation.md`, section 2). Canonicalization is a determinism requirement, not a security boundary: the signature covers the file's bytes, verification never re-serializes, and a canonicalizer defect can therefore make a record fail step 1 but can never make two different records verify as one. DSSE's argument against signing canonical JSON (sign the payload bytes, never a re-encoding) is met here the same way, and the RFC's own test vectors sit in the conformance suite as a determinism check, not a signing check.

The subject digest set carries the git tree id and the recorded inventory digest as separate claims. Identity kind is `worktree`, `index`, `commit`, or `tree`; repository object format is explicit. Equal source identity does not imply equal inventory or model identity when classification or analysis inputs change. An uncaptured input affecting resolution, execution, policy, or validity prevents authoritative acceptance; it is not merely a footnote in a green receipt.

A protected gate signs its receipt with its own SSH key, under `warrant-receipt@<domain>`, which authenticates the producer. A claimed `mode: gate` string alone has no authority. Check receipts use a distinct check predicate and may be unsigned or agent-signed; they satisfy no gate obligation. A replay (`--now` supplied) is `mode: replay`, never authoritative. The accepting system validates the signature, exact candidate, protected configuration and current validity before accepting it. Predicate URIs and namespace domain remain D2; examples are placeholders, not claims of domain ownership.

A gate receipt maps deterministically onto a SLSA Verification Summary Attestation (predicate type `https://slsa.dev/verification_summary/v1`): the VSA's `verifier` is the receipt's `tool` (name, version, binary digest); its `policy` is the receipt's policy digest with the policy directory as `uri`; its `inputAttestations` are the receipt's evidence receipts, report digests, and rulings; its `timeVerified` is `evaluated_at`; its `resourceUri` identifies the repository and tree in the form D2 fixes; its `verificationResult` is `PASSED` when acceptance is `accepted` and `FAILED` otherwise; and `verifiedLevels` is empty because Warrant asserts no SLSA level. The mapping is normative from v1 and is a conformance fixture. `warrant receipt export --vsa <receipt>` emits the companion from the receipt's bytes as a derived, separately signed document (D13 says whether it ships in v1). The VSA is an index entry for stores that already consume VSAs: it carries the binary result and nothing else, and it never replaces the receipt, because the four facets, the findings, the denominator, and the approval items are exactly what it cannot express (`docs/research/2026-09-16-prior-art.md`, section 6).

### 10.5 Verifying a receipt

`warrant verify <receipt.json> [--trust-root <allowed_signers>] [--recompute]` checks, in order, and reports each step's result:

1. Bytes and schema: the file parses as a Statement with a known `predicateType`, is byte-identical to its own RFC 8785 canonical form, and its digest equals its id.
2. Subject: the tree exists in the local repository (skipped with `--offline`, reported as unchecked).
3. Policy binding: the policy files at that tree hash to the recorded blobs and compile to the recorded policy digest.
4. Inventory binding: with `--recompute`, the inventory at that tree recomputes to the recorded digest; without, the recorded summary is reported as unverified.
5. Model binding: with `--recompute` and the pinned integrations installed, the model digest recomputes; otherwise unverified.
6. Instruments: recorded identities match the lock at that tree.
7. Evidence: required test receipts, report files, and ruling artifacts must be present for a fully verified result. A missing artifact is `unchecked` during inspection and cannot authorize acceptance. Check their digests, source identities, execution bindings, authenticated producer identities, and approved runner profiles under section 9.2. A mismatched or unbound subject fails. Verify every ruling's kind-scoped signature, approved-policy chain, supersession, and candidate scope. Check key validity at signed time and current revocation, and rule expiry at both recorded and current evaluation time. A previously valid but now expired receipt is `stale`.
8. Consistency: the facets recompute from the recorded findings, evidence statuses, and approval items.
9. Producer signature: if present, verifies against the trust root under `warrant-receipt@<domain>` with the principal the receipt names; an invalid signature is `failed`, an absent one makes the result `advisory`.

The result is `verified`, `verified-with-unchecked`, `stale`, `advisory`, or `failed`, naming every skipped or failed check. Inspection may examine unsigned documents, but an acceptance consumer checks producer authority first and requires all acceptance-relevant evidence bindings. A `verified-with-unchecked` document is not an acceptance credential. Verification authenticates a trusted runner's assertions; it cannot independently establish test adequacy or rule out a compromised trusted runner. No model is consulted (W24).

### 10.6 Human rendering

When stdout is a terminal, or with `--format human`, the verdict renders as the paragraph the vision promises: what changed since the base (modules touched, capabilities affected), which obligations were satisfied and by what, what is uncertain (the analysis reasons and stale evidence), and whether any permission widened, followed by the primary findings with their focused checks. It is a rendering of the JSON; it contains nothing the JSON does not.

### 10.7 Considered and rejected

- A single pass or fail. The predecessor's `VerdictStatus::{Pass, Fail}` is exactly the collapse the vision forbids.
- A `warning` status. It is how "the parser failed on three files" became a green run.
- Distinct exit codes per facet. Rejected in section 10.2.
- A bespoke receipt format.

  in-toto's Statement costs nothing and lets a receipt sit beside SLSA provenance and a VSA in the same store. What it buys is the schema and the subject and predicate conventions, not a verifier: a Warrant predicate under a detached sshsig verifies with `warrant verify` and with `ssh-keygen -Y verify`, and with nothing shipped by in-toto, cosign, or GitHub until the envelope interface carries it in DSSE (section 11.2, D14).
- A VSA as the receipt.

  It collapses the result to `PASSED` or `FAILED`; the four facets, the findings, the denominator, and the approval items are what it cannot carry. The receipt is the record and the VSA is the derived index entry (section 10.4).
- Asking a model whether a receipt looks right. Never.

## 11. Rulings, signing, exceptions, and policy change

### 11.1 The ruling record

A ruling changes approved policy or accepts specific known violations. v1 has two kinds, `amendment` and `exception` (R15). It is an in-toto Statement, like a receipt, so one canonicalization, signature carrier, and verification path serve both. Its subject binds the relevant policy digest and its predicate names the scope and kind inside the signed bytes. A ruling cannot be relabeled after signing. Trust-root administration is outside Warrant; no ruling bypasses the four-fact acceptance requirement.

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [ { "name": "policy", "digest": { "sha256": "…" } } ],
  "predicateType": "https://warrant.dev/attestation/ruling/exception/v1",
  "predicate": {
    "id": "r_2026-09-18-clone-exception-3",
    "items": [{
      "statement": "These retry-backoff implementations remain separate until the ingest rewrite; consolidating now would couple their release cadence.",
      "owner": "trey",
      "scope": { "modules": ["worker.ingest", "worker.send"] },
      "findings": ["f_3n9q…", "f_8k2w…"],
      "occurrences_digest": "sha256:…",
      "applies_to_count": 2,
      "disposition": "temporary-debt",
      "candidate_range": { "exact_tree": "sha1:71ab…" },
      "expires": "2026-12-01T00:00:00Z"
    }],
    "supersedes": null,
    "rendered_digest": "sha256:…",
    "signer": "trey",
    "drafted_by": "claude-code:fable:session-7a19",
    "created_at": "2026-09-18T15:02:00Z",
    "signed_at": "2026-09-18T15:40:12Z"
  }
}
```

The two kinds have distinct predicate types and sshsig namespaces:

| Kind | Predicate type | Namespace |
|---|---|---|
| `amendment` | `ruling/policy/v1` | `warrant-ruling-policy@<domain>` |
| `exception` | `ruling/exception/v1` | `warrant-ruling-exception@<domain>` |

OpenSSH's `namespaces=` matching prevents an exception-only key from signing a policy amendment (`docs/research/2026-09-16-signing-and-attestation.md`, section 4.2, observed). Keeping these namespaces does not require a Warrant delegation system. Unsupported ruling kinds fail validation rather than being treated as amendments.

An `amendment` carries `policy_digest_before`, `policy_digest_after`, the classified changes, candidate scope, and a typed payload. `policy-change` changes rules; `instrument-adoption` also binds the before/after instrument identities and upgrade report; `accepted-limitation` names the unsupported feature, original gap, reduced requirement and denominator, and expiry. Combining changes retains every applicable payload requirement. A limitation is a visible change in the claim, never evidence that the original claim was proved. At expiry the limitation authorizes nothing and cannot silently leave the reduced requirement active.

An `exception` contains a nonempty `items` array, even for one item. Each item enumerates evaluated findings, their occurrence identity and count, scope, applicable policy, disposition, reason, candidate range, and required expiry or review date (section 11.4). The shared Statement subject binds the applicable policy; items cannot select another policy. One signature covers the exact array, not future items sharing a cause. This is the same schema for a single exception and an onboarding batch.

`drafted_by` is informational. `signer` must verify against the protected trust root. `candidate_range` supports an exact source tree or `descendant_of_commit` with an actual commit id. Trees have no ancestry. An ancestry-scoped ruling requires an independently verified candidate commit-to-tree binding and ancestry check; a bare supplied tree cannot satisfy it, and branch names are labels, not authority. Policy amendments can cover later code under the same rules without repeatedly signing each tree. Exceptions also bind finding identity, count, scope, and the applicable policy.

`expires` is required for temporary-debt items and accepted-limitation payloads. `created_at` is informational. `signed_at` is written by the trusted signing tool inside signed bytes; expiry uses the protected gate clock. This prevents an agent from choosing signing time but does not provide an independently trusted timestamp. A compromised signing key can sign a false date; revocation applies regardless of claimed `signed_at`, and affected rulings require fresh approval from an uncompromised authority.

### 11.2 Draft, render, sign, verify

- `warrant rule draft --kind amendment|exception ...` prepares unsigned rulings.

  It writes a new file with `status: draft`; any principal, including an agent, can run it. `--kind exception --from <receipt>` creates an exact itemized batch of known findings for human review (section 11.4); it does not approve them.
- `warrant rule render <id>` prints the effect summary the signer reads:

  for an amendment, the classified policy diff and every typed payload, including an instrument's corpus report or a limitation's original gap and reduced claim; for an exception, each covered finding, reason, occurrence and count bound, disposition, and expiry or review date. It shows the total and any truncation rather than hiding batch items behind their shared cause. The render is deterministic and its digest is recorded in the signed record as `rendered_digest`, so that "what did I read when I signed" is answerable later.
- `warrant rule sign <id> --key <path>` is human-only signing.

  The trusted signing tool sets signed time, canonicalizes the record, renders its effects and classification from those bytes, and asks the human to confirm. It invokes `ssh-keygen -Y sign` with the kind's namespace and writes the detached signature. Warrant never implements private-key operations, and the signing environment never executes candidate code or plugins. Agents never receive the key or access to its signing service.

- `warrant rule verify <id> --trust-root <allowed_signers> [--revoked <revoked_keys>]` verifies authority and applicability.

  In-process verification must match the SSH oracle on S4's valid and invalid fixtures. It checks the principal, kind namespace, signed-time validity window, current revocation, expiry, canonical policy digest, verified candidate scope, and supersession chain. A missing superseded record or invalid chain fails. Multiple signature files can implement an explicitly approved threshold; this does not create a Warrant account system.

Any byte change to a signed record invalidates it. There is no edit; there is a new record with `supersedes` set, signed again. The old record stays in the repository as history.

One supported handoff uses Git to transport the draft and signed record. The human fetches the proposed revision and reads the named draft blobs without checking out or executing candidate code; a trusted Warrant installation renders and signs those exact inputs, and the human returns the final record and detached signature. The signing environment uses trusted Git configuration and never loads candidate hooks or plugins. A commit signature alone is not the ruling signature. An agent-posted paragraph is a convenient preview, not a substitute for the trusted signing render. This is a workflow using existing tools, not a required shared directory or a fixed number of commits inside Warrant.

Signing and verification go through one interface, the envelope, with one v1 implementation: `sshsig-detached`, the canonical file plus `<id>.json.sig`, produced by `ssh-keygen` and verified in-process with `ssh-key`. The interface is `sign(bytes, kind) -> signature artifact` and `verify(bytes, artifact, trust root, kind) -> principal or failure`, and each record's signature index names the envelope that carries it. DSSE (a JSON envelope whose payload is the same Statement bytes under `payloadType: application/vnd.in-toto+json`), cosign bundles, and GitHub `actions/attest` are conforming envelopes for a later version over the same predicate; a verifier that lacks an envelope reports `envelope-unsupported`, never `failed`, and the predicate never changes shape per envelope. What v1 does not claim: that a Warrant record verifies with in-toto, cosign, or `gh attestation verify` as shipped. It verifies with `warrant verify` and with `ssh-keygen -Y verify`, and the documentation says so in those words (D14).

### 11.3 Trust roots and human administration

The trust root is an OpenSSH `allowed_signers` file: one line per principal with the key and a `namespaces=` pattern list naming the kinds that principal may sign (`warrant-ruling-*@<domain>` for a full signer, `warrant-ruling-exception@<domain>` for an exception-only signer, `warrant-receipt@<domain>` for a gate runner), with `valid-after` and `valid-before` where rotation is planned. Beside it sits an optional `revoked_keys` file in OpenSSH's revoked-keys format. Both are supplied to the gate by `--trust-root` or `WARRANT_TRUST_ROOT`, defaulting to `${XDG_CONFIG_HOME:-~/.config}/warrant/allowed_signers` on the gate user, from a location the candidate cannot write (HC5): a file on the CI runner owned by a user the agent is not, a secret materialized at job start by a workflow file that branch protection keeps agents from editing, or the signer's own machine when the signer runs the gate. The copy at `warrant/allowed_signers` is documentation; `warrant gate` refuses to read it as the root and says so, and a mismatch between the repository copy and the gate's root is reported as a signal so that tampering with the copy is visible rather than silently ignored. The gate needs OpenSSH 9.1 or newer for the `verify-time` option; section 19 records the floor.

The human administers this protected root, revocation list, and approved-policy anchor through existing system or CI controls. Warrant does not rewrite its own trust root, enroll accounts, or manage scoped delegation in v1. Every verification uses the current admitted configuration and records its identity; candidate files cannot install a new root or resurrect a revoked key. Rotation can retain the old key with `valid-before` and admit the new key with `valid-after`. Historical key-window checks use signed time, while current revocation still overrides a claimed earlier date. Configuration changes belong in the operator's existing audit history, not an agent-editable repository copy.

There is no `override` ruling or bypass-to-green mode. A human may ship without Warrant acceptance, but the receipt remains blocked under the unmet requirements. A signed amendment can deliberately change those requirements, and a valid exception can accept a known violation; the resulting record shows the changed rules or disposition. Neither action turns absent execution, incomplete analysis, an invalid signature, or unstable source into a verified fact. Full trust-root transition and scoped-delegation workflows are deferred until a real multi-signer use requires them.

### 11.4 Exceptions

An exception item satisfies a specific evaluated obligation for enumerated findings by human disposition, not by stronger technical evidence. It carries a disposition: `false-positive` (the finding is wrong; the contract or integration should be fixed, and the item records the defect), `accepted-design` (the finding is right and intended; carries a `review_at` date), or `temporary-debt` (the finding is right and will be fixed; `expires` is required). Identity binds to finding ids (section 8.6), scope, an occurrence-evidence digest, and `applies_to_count`. The occurrence digest binds the canonical checked subjects and matched evidence, not only display line numbers or the count. A third clone does not inherit a two-clone exception, and a replacement occurrence does not inherit one merely because the count stayed two. Identity mapping must preserve the covered meaning within the supported check; uncertainty requires fresh review (W16).

The gate evaluates exceptions on its own clock. An expired exception satisfies nothing and is listed under `approval.items` as `expired-exception`. `warrant rule stats` reports exceptions by disposition and age, exceptions whose findings have disappeared (candidates for pruning by a superseding ruling), exceptions whose scope widened (a finding that now covers more instances than the count bound), and growth over time.

There is no baseline (HC7). `warrant rule draft --kind exception --from <receipt>` can draft an onboarding batch, grouped by cause for reading but containing individually enumerated findings, occurrence identities, counts, reasons, owners, scope, and expiry. The human completes missing reasons and selects the intended items; an incomplete item cannot be signed. One signature binds the exact nonempty item array and applicable policy. Re-running the command never refreshes an old signature or accepts future findings. An expired or inapplicable item covers nothing even when other items remain valid. Every report distinguishes accepted debt from fixed code.

A measured floor (a mutation score, a coverage figure, a bundle size) is an ordinary `evidence` requirement with a threshold. It satisfies only that measurement obligation, never a finding's acceptance. Missing or stale required execution remains missing or stale; reducing the evidence requirement requires an explicit policy amendment rather than an exception pretending the test ran.

### 11.5 The policy-widening classifier

`warrant policy diff --base <rev> --candidate <worktree|index|rev>` compares canonical rule definitions and separately reports their effects on the chosen source. It is an advisory comparison. The authoritative gate compares against its protected approved-policy anchor, never a base selected by the candidate (W14).

Dimensions compared, each with a partial order on permissiveness:

- The set of contracts (removal of a contract is widening; addition of an invariant is narrowing; addition of a `preference` with `on_violation: note` is restructuring).
- Per contract:

  selector expressions, allow and deny expressions, `include_type_only`, `dynamic_loads`, `consumers` and `only_via_entry`, permitted callers, writers, required links, patterns, typed claims and required capabilities, evidence scenarios and accepted environment classes, `on_violation`, enforcement, and expiry. Today's resolved subjects are illustrative impact, not the meaning of the rule for future code.
- Declarations: registries (removal is widening: fewer entrypoints in the denominator), stores, external consumers, loaders.
- Exceptions: the set of valid exceptions and their counts and expiries.
- The manifest: `inventory.unknown`, `snapshot.max_file_bytes`, generated and vendored declarations, framework adapters.
- The instruments lock, including Warrant's own version.

Classes: `narrowing` means the accepted set of possible programs becomes strictly smaller; `widening` means it becomes larger; `restructuring` means identical meaning under the supported rule grammar; `uncertain` means the comparison cannot prove one of those relationships or changes combine both directions. Uncertain receives widening treatment (HC6).

The initial classifier proves only supported transformations: identical canonical rules after file movement or ordering changes, literal set inclusion on unchanged scopes, adding or removing independent requirements, and monotone threshold changes under an unchanged measurement definition. Changes to selectors, recognition patterns, interacting overrides, or instrument semantics are uncertain unless a specific tested comparison rule proves them. Matching today's obligation sets or finding counts cannot establish equivalence for future code. A glob changed from all source to today's exact file list must not be called restructuring merely because both match the same files today.

The output shows the rule-level classification, its proof rule or uncertainty, and a separate snapshot-impact example. Any semantic replacement needs an amendment binding the approved before and proposed after digests. A mixed narrowing-and-widening change is overall uncertain while preserving its individual changes. Unchanged canonical rules do not need approval merely because source changed which obligations apply. Formatting-only changes preserve the policy digest. A signed amendment for different rules never approves the current edit.

### 11.6 Engine defaults

A new Warrant or instrument identity requires explicit adoption even if the frozen corpus output is unchanged. The corpus provides regression evidence, not a proof that semantics are identical for all programs. Defaults are explicit canonical-policy inputs; changing them is classified like any rule change. Release notes distinguish enforced semantics from new flags (W31).

### 11.7 What the mechanism does and does not prevent

| The agent can... | Prevented by | Not prevented |
|---|---|---|
| Edit `warrant/allowed_signers` in the repository | The gate ignores it; the root comes from outside (HC5); the mismatch is a reported signal | |
| Edit a signed ruling's JSON, or reformat it | Signature verification fails; bytes are compared, never re-serialized | |
| Choose a draft date to make a ruling look current | Trusted signing sets signed time; current gate time enforces expiry and current revocation still applies | A compromised signer or signing key can lie about signing time; there is no independent timestamp proof |
| Sign an exception with an exception-only key and label it a policy amendment | The namespace is per kind; the trust root's `namespaces=` list scopes the key; `ssh-keygen` refuses | |
| Copy an old signed ruling to cover a new finding | Finding ids, count bounds, `candidate_range`, and `policy_digest` are inside the signed bytes | |
| Write "approved by Trey" anywhere | Nothing reads prose for approval | |
| Fabricate a test receipt | Every gate-credited class requires authenticated provenance from an admitted protected runner, exact execution bindings, and parsed report results | An authenticated runner can still run an inadequate test; recomputing hashes proves no execution |
| Regenerate the model cache | Gate recomputation or authenticated artifacts bound to all analysis inputs; agent-writable cache rows are not trusted | Content addressing alone cannot authenticate the computation |
| Replace the Warrant binary on the agent's machine | Checks are advisory; protected gate installation, launcher, and authenticated producer identity decide acceptance | A self-reported binary digest is not evidence of which binary ran |
| Sign with the signer's key | The key is on another Unix user with a passphrase | A compromised signer machine |
| Convince the signer to sign without reading | `rule render` and the receipt's `rendered_digest` make "what did you sign" answerable | A signer who does not read |
| Write a test that asserts nothing | Signals count assertion changes; the taste profile can ask a judgment | Nothing proves a test is adequate; the receipt says what ran, not that it mattered |

### 11.8 Considered and rejected

- The predecessor's baseline entries with an `owner` field. Text in the working tree, editable by the implementer; the exact failure W15 describes.
- Signed git commits or tags as the approval channel.

  A commit signature binds a person to a tree, not to a scoped statement with an expiry and a policy digest; it cannot express "this exception, for these two findings, until December"; and it approves everything in the commit, including the agent's other changes.
- GPG. The mechanism works; the key management and user experience do not, and every developer already has an SSH key.
- Sigstore keyless signing with Fulcio and Rekor.

  Strong for teams with an identity provider and a public transparency log; heavier than a solo maintainer needs, and it introduces a network dependency into signing. It is the documented path for teams later; the ruling format does not preclude it.
- A user and role database inside Warrant. W15 says use an existing trusted mechanism; the allowed-signers file and the CI runner's secret store are that mechanism.
- One namespace for every ruling kind (`warrant-ruling@v1`, the first draft of this section).

  Per-kind namespaces cost nothing and give kind restrictions with OpenSSH's own `namespaces=` matching; the first draft would have needed Warrant code to enforce what the trust root can already say.
- DSSE envelopes as the v1 default.

  A DSSE payload cannot be verified by stock `ssh-keygen`, and the detached sshsig beside a canonical file can. DSSE's reason to exist, signing the payload bytes rather than a re-encoding, is met here another way, because verification compares bytes and never re-serializes (section 10.4). DSSE is a conforming envelope over the same Statement behind the envelope interface (section 11.2), for teams that already verify DSSE bundles, and D14 records the choice.
- RFC 3161 timestamps.

  They prove a signature predates a revocation without trusting either clock, which matters only after a key compromise, when re-signing the rulings that still matter is the right response anyway; a token field is reserved in the signature index for later.
- A drafter-supplied `signed_at`. It is the backdating hole git has (validity is checked at the committer's own timestamp) and it costs one line to close.
- Seven ruling kinds for the first signer. R15 retains amendments and itemized exceptions; typed amendment payloads preserve instrument and limitation requirements without separate approval systems.
- An emergency green receipt for an unevaluated candidate. A human's decision to ship is not evidence that unchanged acceptance requirements were met.

## 12. Interfaces for agents

The CLI returns architectural answers, diagnostics, and verifiable records. MCP and hooks expose those same operations; they add no work-management concepts.

### 12.1 The command set

| Command | Result | Writes |
|---|---|---|
| `warrant snapshot --worktree \| --index \| --commit <rev> \| --tree <oid>` | Exact source manifest | cache |
| `warrant inventory` | Classified inventory with omissions | cache |
| `warrant model [--capabilities]` | Program model and supported analysis claims | cache |
| `warrant query <question> \| --sql` | Architectural facts from the model | nothing |
| `warrant context <description> \| --paths \| --symbols` | Relevant architecture before an edit | cache only |
| `warrant propose --description \| --patch \| --paths` | Architectural impact of one proposed change | temporary snapshot/cache only |
| `warrant check [--snapshot <snapshot>] [--changed] [--index] [--base <snapshot>] [--compare <receipt>]...` | Advisory verdict with optional named source or result comparisons | cache, check receipt |
| `warrant gate --candidate <snapshot> --gate-config <protected-config>` | Authenticated full verdict on supplied source | protected receipt store |
| `warrant explain <finding \| cause> [--show-source]` | Evidence, affected consumers, and possible remedies | nothing |
| `warrant verify <receipt>` | Verification result with skipped checks explicit | nothing |
| `warrant policy compile \| lint \| effective \| diff` | Rule compilation, explanation, or comparison | cache only |
| `warrant rule draft \| render \| sign \| verify \| list \| stats` | Amendments or itemized exceptions; `draft --kind exception --from <receipt>` supports exact batches | named draft/signature outputs; signing is human-only |
| `warrant attest run -- <cmd>` | Evidence recorded from an existing tool | execution outputs, receipt |
| `warrant evidence import \| list` | Bound evidence or explicitly untrusted import | cache |
| `warrant instrument status \| qualify \| upgrade` | Identity checks or before/after reports; upgrade does not adopt a version | cache, named report output |
| `warrant census observe \| propose \| import` | Observation without existing policy; starter manifest and policy proposals; import only for supported formats | cache; new proposal files on explicit request |
| `warrant map [--scope] [--diff]` | Mermaid or JSON view of the same model | nothing unless caller redirects |
| `warrant serve --mcp` | Stdio access to advisory/read operations | cache, check receipts only |
| `warrant hook <harness> [--install]` | Supported before-decision context delivery and diagnostic feedback, with coverage and availability explicit | harness config only on explicit install |
| `warrant selftest \| self-qualify` | Installed-control checks or build comparison on the frozen corpus | isolated fixture outputs, report |
| `warrant judgment advise` | Optional, explicitly enabled typed model advice | advice cache; may use network |
| `warrant schema <name>` and `warrant capabilities` | Schema and implemented features, including unsupported adapters | nothing |

A command not on this list needs a section 20 ruling to exist. No command edits application source, creates work items, installs packages, merges branches, or adopts policy. Census and ruling commands write proposals to caller-selected new files and refuse existing destinations. Source changes for `propose --patch` and self-test are confined to disposable analysis fixtures. The `attest` command runs caller-supplied tools only under the execution distinction in section 9.2; it does not gain permission to run them as the human signer or gate controller.

### 12.2 The output contract

- JSON is the default off-terminal; NDJSON streams large results; human output renders the same data. No ANSI in machine output.
- Every emitted document has a published versioned schema.

  Stable ordering applies to semantic records. Runtime timestamps and new model opinions are explicitly variable; byte-identical output is promised only for identical complete inputs, including evaluation clock where relevant.
- Lists support limits and cursors with `truncated`, `total`, and `next_cursor`. A summary never claims unreturned findings do not exist. Pagination limits rendering, never the gate's internal evaluation or receipt evidence.
- Findings carry locations rather than source bodies. `explain --show-source` is an explicit bounded read. Each explanation field can link to a longer explanation instead of silently cutting evidence.
- Errors name the code, reason, relevant locations, and next useful diagnostic. Invalid input produces no acceptance receipt. Missing required evidence during a valid evaluation appears in the four-fact verdict.

### 12.3 Context

`warrant context` accepts a description, paths, or symbols and answers what already exists and what governs it. It uses the description for retrieval without persisting it as a task or prompt log. The initial retrieval path works offline: exact symbols and paths, declared responsibility names and aliases, intent text, then graph neighbors and relevant examples. Optional semantic retrieval can suggest candidates, but never certifies ownership or changes compliance.

Prompt text, paths, and source stay local unless the user explicitly enables an identified service and its allowed data scope through runtime configuration. Editable repository metadata alone cannot authorize export. A method that calls a service names it in `retrieval.methods`; cached model facts do not require storing prompt text. The harness or caller may retain its own transcript, which is outside Warrant's storage promise.

Every candidate has a reason for inclusion and source references. Declared owners, observed consumers, authenticated test observations, and inferences have different labels. A representative example points to actual source and the evidence available for it; similarity alone does not make it a recommended implementation. Tests on older source are identified as historical examples, not current proof.

Results identify the requested source and policy separately from those actually used, plus the model-input identity (section 3.5). `model_status: current` means the model matches the requested snapshot, policy, dependencies, and analysis configuration, not necessarily the caller's worktree. `stale` labels a last-known result when those inputs differ or freshness cannot be established; all graph-derived answers remain stale because an unchanged file can depend on changed inputs elsewhere. Observed changed paths help explain the mismatch but are not a complete invalidation proof. `unavailable` returns no model facts and a reason. A hook never waits for a full cold rebuild merely to pretend its last-known model is current.

`--budget <bytes>` bounds the complete encoded output, including identity and omission metadata. Capped results identify omitted scope without cutting a JSON value or hiding truncation. A budget too small for the minimal status document returns an explicit error; adapters reserve enough room for that status and their own framing.

```json
{
  "schema_version": "warrant.context/1",
  "query": "add grouped undo to the meeting debrief",
  "snapshot": "sha1:...",
  "policy_digest": "sha256:...",
  "requested_snapshot": "sha1:...",
  "requested_policy_digest": "sha256:...",
  "model_inputs_digest": "sha256:...",
  "model_status": "current",
  "retrieval": { "status": "matched", "methods": ["declared-vocabulary", "graph-neighbors"], "candidates_total": 2, "returned": 2, "truncated": false, "qualified": true, "qualification_report": "sha256:..." },
  "facts": [
    { "kind": "owner", "subject": "core.actions.undo", "basis": "declared", "source": "contract:core.actions.undo", "text": "Owns compensation registration." },
    { "kind": "interface", "subject": "core.actions.undo::registerCompensation", "basis": "observed-static", "source": "model:symbol/42", "text": "Three observed instances consume this interface." }
  ],
  "examples": [{ "symbol": "core.commands.debrief::recordDebrief", "path": "packages/core/src/commands/debrief.ts", "basis": "observed-static", "reason": "Uses the declared compensation interface", "evidence_status": "historical-only" }],
  "suggestions": [{ "basis": "inferred", "text": "Grouped undo may reuse the existing compensation owner; check whether its responsibility is the same." }],
  "affected_consumers": ["worker.workflows.debrief"],
  "contracts_applicable": ["capability.undoable-operation"],
  "evidence_required": [{ "tag": "undo-denied:{instance}", "class": "integration", "scenario": "denied-request-produces-no-effect" }],
  "limitations": ["Binding references do not establish all runtime callers."]
}
```

Retrieval reports `matched`, `ambiguous`, `no-match`, or `incomplete`. Ambiguous returns alternatives and the missing discriminator; no-match says no candidate was found by the named methods, not that the repository has no owner. Incomplete identifies unindexed scope, unsupported analysis, or truncation. A conversational prompt unrelated to the code should return no-match without irrelevant candidates; S7 counts misleading answers on those prompts separately from missed owners. Resolution, ownership, and retrieval completeness are separate.

S7 qualifies retrieval before M1 closes. `retrieval.qualified` is false until an applicable report meets the predeclared acceptance criteria; the existence of a report, including a failing one, is not qualification. A qualified result identifies that report and its measured scope. Tests cover vocabulary mismatch, multiple plausible owners, genuine novelty, unrelated prompts, stale examples and inputs, no model, and capped output. Known aliases live with domain declarations, not in a hidden synonym database.

### 12.4 Propose

`warrant propose` takes one intended change and reports relevant contracts, observed consumers, possible reuse, evidence requirements, and proposed rule changes. A supplied patch is applied exactly to its named base in a temporary snapshot; mismatched context is rejected rather than fuzzily applied. Without a patch, impact remains conditional on the described change and cannot be represented as a verified violation.

Reuse candidates are labeled `exact-name`, `declared-responsibility`, `structural-similarity`, or `semantic-suggestion`. A matching name is not proof of duplicate behavior. An existing signed ownership declaration is architectural intent, not proof that no other code implements the same behavior. The useful result names the existing interface, its consumers, and why it may fit; it leaves legitimate new architecture possible. Proposals have no registration, status, assignment, deadline, or dependency graph. No multi-proposal overlap mode exists.

### 12.5 MCP

The stdio MCP server exposes the same advisory/read operations: context, query, propose, explain, check, verify, effective policy, and map. Results use the CLI schemas. Cache and check-receipt writes are disclosed; no source mutation, signing, policy adoption, or gate execution is exposed. An acceptance system invokes the protected gate independently. The first complete CLI experience does not wait for every transport adapter.

### 12.6 Hooks: delivery and feedback

The map must reach an agent before the decision it is meant to inform. Context attached to the result of an already-selected tool call can improve the next decision, but cannot change the edit that just executed. v1 starts with a bounded map summary at session start and a bounded `warrant context` result beside the submitted prompt, on a harness whose before-decision placement has been verified. An ordinary complete no-match may stay silent; timeout, stale, unavailable, and incomplete are not treated as no-match. The adapter records delivery status without logging prompts. Warrant stores no session progress or delivery history; any ephemeral deduplication belongs to the harness and is keyed to the source, policy, and context actually delivered.

After supported editing events, harness and Git hooks invoke changed-source diagnostics. They do not edit, restage, install packages, or approve anything. Temporarily unparseable code returns `mode: diagnostic` with provisional parse errors and affected scope, not a clean verdict. Normal checks and the gate still treat required parse failures as incomplete analysis. Hooks are guidance and feedback, never the enforcement boundary.

A shipped delivery adapter must demonstrate:

1. Context appears in model input before the relevant decision, established by an observable harness run, not merely a hook process starting before a write. Installation, trust review, and a successful sentinel delivery are separate facts.
2. Delivered context identifies requested and used source/policy inputs, freshness, retrieval status, and omitted scope under section 12.3's output budget.
3. The context subprocess has a deadline shorter than the harness timeout so the adapter can normally return a small failure status. If the harness skips, kills, distrusts, or fails to invoke the adapter, delivery is unavailable; no warning from code that never ran is promised. Qualification includes these cases and the harness-visible evidence of nondelivery. No timeout produces an acceptance receipt.
4. The adapter names the events and tool families it supports. A file-tool matcher does not establish coverage of arbitrary shell writes. Prompt-submit guidance can precede either kind of edit, but it is not file-targeted interception of every writer.
5. No task record, hidden work state, prompt log, or implicit remote disclosure is introduced. Privacy follows section 12.3. Noise on unrelated prompts, repeated context, output caps, and cold-model failure are measured rather than assumed harmless.

Current [Claude Code hook documentation](https://code.claude.com/docs/en/hooks) and its [guide](https://code.claude.com/docs/en/hooks-guide), and the [Codex hook documentation](https://developers.openai.com/codex/hooks), were consulted on 2026-09-17 alongside local versions Claude Code 2.1.275 and Codex 0.154.0. They document session-start and prompt-submit context delivery. Nonblocking pre-tool context does not establish an opportunity to reconsider the pending call; tool matching, output limits, and trust admission differ by harness. No Warrant adapter or runtime delivery test exists yet. M1 records exact supported versions and tested events; one verified adapter is sufficient for the early comparison, not a promise of every transport.

A deny-with-context experiment may cancel a pending edit and allow the model to reconsider. It is not the default: cancellation is stronger than ordinary guidance and needs harness-owned duplicate suppression to avoid loops. Only a separately specified comparison arm can establish whether that interruption improves outcomes. `warrant hook <harness>` emits configuration; `--install` changes it only on explicit request, and cannot grant native hook trust on the user's behalf.

### 12.7 Considered and rejected

- Work registration and overlap reports, even if another tool would consume them. They resemble work management and violate R11.
- Remediation packets, dependency ordering, and Beads scripts. Findings and explanations already supply the facts other tools need.
- Separate human and agent truth. Human output renders structured records.
- Additional hook/transport adapters merely for completeness. Support them when the first user's workflow needs them.
- LSP in the initial release. Findings retain locations and related information so a later adapter can reuse them.

## 13. Different source, different evidence

### 13.1 Source comparisons, not work coordination

External Git and agent tools supply the candidate and any source revisions to compare. Warrant has no registered lanes, knowledge of who is working, or merge-order state. Each supplied tree gets its own inventory, model, and result. The source comparison base is distinct from the protected approved-policy anchor.

### 13.2 Before editing

Context and proposal queries report architectural relationships in the selected source: shared interfaces, affected consumers, registries, and evidence requirements. Other tools may use those facts when coordinating work. Warrant does not ingest assignments or infer work collisions.

### 13.3 Evidence validity

A test or instrument execution receipt is valid for the exact source and execution inputs it names. A different combined tree requires fresh required evidence in v1. A graph of imports is not a complete graph of test inputs: fixtures, generated outputs, migrations, configuration, services, and environment can matter without being imported. There is no carry-over flag or cross-tree execution-evidence reuse in v1.

Warrant names the mismatched source and changed inputs it can observe, without claiming that its changed-file list exhausts the reason a test might differ. An identical tree may reuse an authenticated receipt only when every required execution/configuration binding and validity condition still matches. A new required scenario, instrument, service recipe, or expired ruling invalidates that reuse even if source is unchanged.

### 13.4 Incremental analysis

Safe analysis caching is separate from reusing test evidence. Cache keys include all inputs listed in section 3.5. Warrant may reuse parsing of identical blobs under identical parser configuration; declaration, resolution, and evaluation caches must include the inputs they actually depend on. A full evaluation is the reference behavior. An affected-scope optimization ships only after full and incremental results match on source, configuration, declaration, instrument, and policy changes in conformance tests.

The gate re-derives acceptance using the current protected policy anchor, trust configuration, evidence set, revocations, and clock on every invocation. Cached acceptance cannot outlive an exception or ignore newly required evidence. Agent-writable cached findings are not authoritative without recomputation or an admitted producer binding.

### 13.5 Findings introduced by combination

`warrant check --snapshot <candidate> --compare <receipt-a> --compare <receipt-b>` supplies the candidate and named comparison results. The caller obtains each receipt by checking its own source revision; Warrant neither combines branches nor registers their work. The output lists every compared source, policy, instrument, scope, and completeness identity. Compare underlying obligations and occurrence/count evidence, not only stable finding ids: an existing finding can gain a new violating occurrence without changing id.

The ordinary claim is `introduced-relative-to-comparisons`, naming the supplied revisions. Stronger combination-only attribution requires independently verifiable derivation of the combined candidate from the named inputs, with any additional resolution edits identified, plus comparable complete results for the relevant obligations under the same approved rules and instruments. Without that derivation, unrelated source revisions cannot prove that combining work caused a finding. Missing inputs, incomparable requirements, or incomplete comparison evidence make attribution `undetermined`. This is a code comparison, never blame or an assignment of work.

### 13.6 External consumers of findings

Findings have stable ids, causes, affected consumers, required checks, and plain-language explanations. They are ordinary versioned JSON. Existing tools can read them without a Warrant-specific task export. Warrant neither creates nor closes work items.

### 13.7 Considered and rejected

- Managing lanes without calling it orchestration. R11 excludes the behavior, not just the label.
- Inferring complete test inputs from import closure. It omits runtime and non-code inputs.
- Trusting cache digests as proof of computation. Digests authenticate bytes only when an admitted producer binds them.
- Holding a repository-wide lock while analyzing. Immutable captured source avoids a work-coordination lock.

## 14. The map

`warrant map` renders the same snapshot-derived model agents query, with declared policy beside observed relationships. v1 emits Mermaid for people and JSON for tools. Modules, interfaces, entrypoints, stores, effects, and external consumers can be scoped to a module and its neighbors. A diff shows added and removed relationships between supplied snapshots.

Observed, declared, inferred, forbidden, and unresolved relationships remain distinguishable. Unread files and unsupported analysis are visible in both formats. A diagram cannot claim completeness that the model lacks. Generated output can be saved by the caller; it is labeled with its source and policy identities, not maintained as a second source of truth.

DOT, standalone HTML, C4, and Structurizr exports are deferred unless a real consumer needs them. Warrant does not maintain a separate diagram model or hosted viewer.

## 15. Census and adoption

The first useful map must not require writing all the policy by hand, nor should it turn the repository's existing accidents into law.

### 15.1 Adoption path

`census observe` runs without a Warrant manifest or existing policy. It discovers supported integrations from existing language and workspace manifests, reporting ambiguous or unsupported units rather than guessing completeness. It inventories source and reports units, observed imports and entrypoints, cohesion, cycles, and possible interfaces. Ownership is explicitly undeclared where no human-approved contract exists. Inventory/readability accounting still runs, but census is not an acceptance mode.

`census propose` writes a starter manifest and draft policy to new caller-selected destinations. Initial module candidates come from workspace/package boundaries, directory groupings, and observed import cohesion, using a deterministic method recorded with the proposal. Each item carries its basis, uncertainty, and `authority: draft`. Directory structure is a grouping hypothesis, not approved architecture. Proposed dependencies and interfaces reflect observed edges and exports; ambiguous ownership remains unresolved. Existing shape is an example to judge, not a promise of zero findings. Effects, capabilities, and required behavior tests need human intent the code cannot supply. Existing files are never overwritten; rerunning census produces a new proposal rather than silently updating approved policy.

A person edits toward intended architecture. Existing `policy lint`, `check`, `explain`, and `rule draft` operations show the resulting obligations and any existing violations. `rule draft --kind exception --from <receipt>` removes repetitive drafting, but every accepted-debt item still has a specific identity, reason, scope, and expiry. It is never a saved finding baseline. Existing planning tools handle code repair independently. A signed amendment adopts the rules; the protected gate can then enforce them.

The minimal observe/propose path ships with the first pre-edit map. It does not wait for importers, a remediation workflow, or full model-assisted naming.

### 15.2 Observed versus intended

The approval rendering shows proposed observations beside the chosen rules and their classified changes. Observed proposal digests are provenance, not authority. A changed grouping is not automatically restructuring: policy equivalence follows section 11.5, and uncertain group changes are shown as uncertain. Re-running census never silently changes approved policy.

### 15.3 Importers and reuse decisions

Three candidate importers translate the same repository's existing rules, not architecture borrowed from unrelated projects:

- dependency-cruiser: existing allowed or forbidden dependencies and the path groups they use.
- eslint-plugin-boundaries: existing element types and boundary relationships.
- Nx: project tags and dependency restrictions, with both the graph and governing rule configuration supplied. A project graph alone does not contain the intended restrictions.

Keep an importer when the first adoption actually needs it and a qualified translation reduces work. Do not promise all three in v1. An existing checker can instead keep running and supply its findings as external evidence; consuming a report and translating its configuration are different capabilities.

Every importer produces drafts, lists unmapped or uncertain semantics, and refuses to silently drop them. Representative source-format fixtures must show that translated checks preserve the intended restrictions before an import can be ratified. Candidate-controlled executable configuration is read in the untrusted evidence environment, never evaluated by the human signer or protected gate controller. A shared library that cheaply supports several needed formats is welcome; writing bespoke readers merely to lower dependency count is not.

### 15.4 Comparing findings after a change

`check --compare <receipt>` reports whether a previously identified obligation is now satisfied by evidence, covered by an exception, still unsatisfied, no longer applicable with a stated reason, or undeterminable. Repeating the argument adds named comparison results under section 13.5's comparability rules. These are evidence comparisons, not repair statuses. A subject disappearing is not automatically a successful fix; missing consumers or entrypoints remain checkable obligations. No packet title, assignee, repair dependency, open/closed state, or task export is produced.

### 15.5 Considered and rejected

- Ratifying whatever produces zero findings. Adoption chooses intent, not silence.
- Owning the cleanup process. Findings are Warrant's output; work management belongs elsewhere.
- Making all three importers prerequisite to the first repository. Compatibility must serve a real adoption need.
- Rebuilding an existing checker because its policy format was not imported. Report ingestion and rule translation are independent reuse choices.

## 16. Optional model advice and the taste profile

Taste remains a profile. Measurable requirements use deterministic contracts and existing instrument reports. Model suggestions remain opinions. The first complete Atlas experience does not depend on a hosted model, and no model decides compliance or signs approval.

### 16.1 The typed advice interface

R8 retains System One's useful shape: structured state plus typed questions, yielding typed answers. The initial adapter supports a yes/no question (`noul`) or a choice among named alternatives, including `cannot-tell` or `none`. Numerical answers are recorded as the provider's estimates, not described as calibrated probabilities without measured calibration. No answer becomes a repository quality score (HC9).

The interface is small and provider-independent. TypeSafe Jev is a candidate backend, not a required service. Reuse a maintained adapter if it meets the need more simply than custom provider clients. Support one useful route first; additional providers, score-type questions, provider comparisons, and a general calibration command suite are deferred.

### 16.2 The profile file

```yaml
schema_version: warrant.profile/1
name: trey
description: Reuse established owners. Prefer modules with substantial behavior behind a clear interface.
measurable:
  contracts: [style.no-default-exports]
  evidence:
    - { instrument: fallow, metric: duplication-blocks, at_most: 0, scope: { modules: ["core.*"] }, consequence: review }
signals:
  suppressions-added: review
  new-state-owner: review
  assertion-count-decreased: note
judgment:
  backend: none
  consequence: note
  questions:
    - id: existing-owner
      type: choice
      instructions: Which declared module may already own the described responsibility?
      criteria_from: module-catalog-with-none-and-cannot-tell
```

Measurable acceptance requirements and review-triggering deterministic signals belong to the signed policy meaning. Preferences advise by default (R16); the explicit `review` entries above illustrate a deliberately stricter choice, not the default or ratified Atlas policy. Advice stays visible with counts. Its settings have provenance and privacy controls, but cannot silently add an acceptance requirement.

### 16.3 State and privacy

State contains the proposed change or hunk, relevant enclosing code, declared intent, applicable contracts, candidate owners, consumers, and source references. Retrieval chooses the relevant scope and reports its omissions. Token or byte limits, truncation, unavailable context, and provider rejection are recorded. A result with incomplete state cannot assert that no relevant owner exists.

Sending source or map context to a hosted provider requires explicit opt-in to that provider and data scope. Credentials are supplied by the existing credential mechanism and never enter repository files or receipts. An optional semantic context query must request network use explicitly; ordinary context, checks, gate, and verification remain offline. `judgment advise` produces a separately labeled advice artifact that an offline renderer may display.

### 16.4 Consequences and outages

Initial model advice has `consequence: note`. It cannot change any acceptance facet. Disabled, unavailable, rate-limited, or malformed model output is reported as missing advice and does not demand a human decision or block code. Warrant never instructs an agent to keep rewriting correct code until a model approves it.

If a later signed policy gives a model question `consequence: review`, a trigger will require human review and therefore block acceptance. Calling that behavior nonblocking would be false even if compliance remains untouched. Such a policy must name the question, calibrated provider/model version, data scope, treatment of unavailable answers, and how a human resolves a review. This capability is deferred pending D3; it is not enabled by the example profile or the mere existence of a threshold.

### 16.5 Backend identity

Advice records the adapter, requested and returned model identifiers, question digest, state digest, response, truncation, and available request metadata. A rerun is a new opinion, not a deterministic recomputation. Unknown or changed provider identity is explicit. The deterministic core builds without remote advice support.

### 16.6 Evidence before expansion

First measure whether advice identifies useful reuse that the offline context path misses, how often its suggestions are wrong, and whether it saves human attention. Only demonstrated benefit justifies more providers or calibrated review thresholds. A hosted service's claimed calibration does not establish calibration on this repository's code or questions. Deferred review-triggering behavior needs repeatability and human-labeled tests before adoption; the initial product needs only truthful advice handling and offline availability.

### 16.7 Considered and rejected

- Calling required approval nonblocking. It blocks acceptance by definition.
- Treating a model outage as a defect in the code. Missing optional advice is not a compliance finding.
- Making model approval the cheapest route to green. Humans decide whether an opinion matters.
- Building a model-provider or evaluation platform inside Warrant before its advice earns that complexity.
- Deleting the typed advice interface entirely. R8 retains it; the scope reduction concerns breadth and authority, not useful reuse.

## 17. Proving Warrant's own claims

A gate that cannot be shown to catch what it claims to catch is a decoration. This section is the evidence discipline Warrant applies to itself.

### 17.1 Conformance fixtures

`tests/conformance/<area>/<case>/` holds controlled fixture repositories, policies, expected semantic findings/facets, and positive and negative cases per hard control. Areas cover inventory, model, context retrieval, typed claims, policy identity and change classification, verdict/diagnostic separation, authority, exact-source execution binding, fresh combined-source evidence, census, supported adapters, offline/model-outage behavior, and bounded output. Supplied Git revisions exercise combination cases without a lane registry. Every case records which check it proves; a fixture cannot certify the real deployment's isolation.

Every fixture runs in CI on every change. The suite seeds from the predecessor's fourteen fixture projects and its adversarial cases, rewritten to the new policy vocabulary.

### 17.2 Breaking the control

Three mechanisms make sure a broken checker is noticed by its own tests (W30):

- Mutation testing. `cargo-mutants` runs over `crates/core` (obligation evaluation, facet derivation, the widening classifier, receipt verification) and `crates/authority` with the conformance suite as the killer.

  A surviving mutant in those crates fails CI. Other crates report mutation scores without failing, to start, with a section 20 decision on when to raise them.
- Fault injection in test builds exercises parse failure, missing instruments, report truncation, expired rulings, unstable source, invalid snapshot input, untrusted configuration, and output truncation.

  Each must reach the correct diagnostic or facet through all output paths. Invalid inputs produce an input error; they do not fabricate a verdict over nonexistent source.
- Adversarial conformance cases test that missing authority or evidence cannot become acceptance.

  Inline approval prose and changed hooks cannot relax protected requirements. An unused census draft does not change approved policy. Parsed findings remain findings even when an instrument exits zero.

Additional acceptance cases required by this revision:

| Case | Required result |
|---|---|
| Authorization call occurs but does not prevent the denied effect | Structure may be present; the required denial-behavior case fails or is missing. No authorization proof is claimed. |
| A recognized ORM write rule omits another write form | The output names the narrower recognized-write claim. A universal ownership claim is unsupported, not satisfied. |
| All capability links exist but a required behavior case fails | Structural presence cannot satisfy behavior evidence. |
| A glob is replaced by today's exact file list | Equal current obligations do not establish restructuring; future scope change remains visible. |
| New ordinary source matches an unchanged approved rule | Policy digest stays stable; new obligations are derived without requesting a new signature. |
| Draft narrowing or an alternate source-comparison base is supplied | Neither changes the protected approved-policy anchor or adopts new rules. |
| Index/commit differs from dirty worktree | Execution uses the selected materialized bytes or cannot supply gate evidence. |
| Source is changed during a test and restored before exit | The approved source-isolation mechanism prevents or detects it; a start/end hash alone is insufficient. |
| Receipt claims a real database, but no admitted service binding exists | Required integration evidence remains missing. |
| Agent-signed local receipt or self-reported checker digest is supplied | Producer admission fails for gate evidence, regardless of claimed class. |
| Candidate-controlled report is stale, skipped, zero-case, or truncated | Required executed cases remain unsatisfied; no success from exit status alone. |
| A combined tree changes only a fixture, generated input, or migration | Evidence from the other tree remains stale. |
| An exception expires while a cached result exists | Current gate acceptance is re-derived and blocked; cached green is not authority. |
| Optional model service is disabled or unavailable | Missing advice is explicit; acceptance facets do not change. |
| A stable finding id retains its count but a covered occurrence is replaced | The old exception does not silently cover the replacement; identity mapping or fresh approval is required. |
| An exception batch gains an item or contains an expired item | Added bytes invalidate its signature; an expired item covers nothing even when other items remain valid. |
| A candidate supplies its own source-comparison base to hide a required signal | The authoritative gate uses the protected base, or reports the required comparison incomplete. |
| A context report exists but missed its declared qualification criteria | Retrieval remains unqualified; report existence is not success. |
| A model's own file is unchanged but another analysis input changes | A stale graph answer is not relabeled current; all required model inputs determine freshness. |
| A hook runs but its context arrives only beside the completed edit | It is feedback, not verified before-decision delivery. |
| The harness skips an untrusted hook or discards its late output | Delivery is unavailable, not silently recorded as a successful context injection. |
| A human ships while required evidence is missing | The unchanged-policy receipt stays blocked; no override-to-green mode exists. |

The protected-runner cases also require a focused check using the actual D5 deployment and safe fixture data before M3 closes. Passing local fixture assertions is not evidence that production credentials and execution are separated.

### 17.3 The frozen corpus

The corpus has three parts and one identity:

- Synthetic labeled fixtures, in this repository under `tests/conformance/`, small and exact.
- Frozen real repositories with their dependency trees, in a separate repository, `warrant-corpus`, as tarballs referenced by digest from `tests/corpus/manifest.yaml`:

  Atlas at a pinned commit (the first customer), Warrant itself at each release, and open-source TypeScript repositories chosen for shape rather than fame (a Next.js application, a pnpm monorepo with project references, a CommonJS library, a Vite library, a repository with path aliases and package exports), with Rust and Python repositories added as those integrations land. Dependency trees are included so that resolution parity is measured against what the build actually sees.
- A held-out Atlas outcome set for section 17.5. Human assessments and agent opinions are labeled separately; disagreements remain visible. A larger model-calibration set is deferred until required review is proposed.

The corpus id is the manifest digest used by qualification. Real Atlas artifacts and dependency bundles remain private unless separately approved for publication. Public fixtures and publishable metadata can live with the public repository; D4 governs storage/publication, and GitHub writes retain their separate authorization gate.

### 17.4 Repository self-test

`warrant selftest` checks installed static controls against harmless mutations in a disposable source copy (W32). Supported templates include a forbidden dependency, interface bypass, removed declared entrypoint, or recognized effect reference outside its permitted scope. It compares with the unmodified result and requires the expected new finding; unrelated existing violations do not make the test impossible. Arbitrary patterns and behavior-evidence requirements without a safe mutation template are listed as unexercised. The result names exercised, failed, and unexercised controls and cannot claim full coverage when any required control was skipped. It never edits the real tree, runs application code, uses customer credentials, or accesses the network. Real behavior-test adequacy and runner isolation require their separate evidence.

### 17.5 Does Warrant improve the code and reduce supervision?

Conformance proves the checker follows its rules. A separate Atlas comparison tests the product's reason to exist. Existing experiment and agent tools conduct it; Warrant does not gain an experiment scheduler or task tracker.

First run a small advisory comparison at the end of M2, after S7 has qualified retrieval and one before-decision delivery adapter, before building M3's protected acceptance deployment. Both groups use the same fixed advisory checker and existing Atlas behavior tests; the context treatment is the intentional difference. These runs provide product evidence, not authenticated Warrant acceptance receipts. R14 is a stop-and-rethink checkpoint: if guidance shows no benefit, revise the guidance and rerun an appropriate comparison rather than automatically proceed to M3. Mixed or inconclusive results are presented to Trey, not converted into a pass by changing the rubric afterward. Proceeding requires his review of the declared usefulness criteria and measured result.

M4 repeats the outcome assessment on held-out requests not used to tune retrieval, policy, or the early guidance. The later comparison includes the complete protected acceptance experience and follow-on maintainability. A passing early pilot is not a substitute for that release evidence, and reusing the pilot's tuned examples is not an independent held-out test.

Before running agents, freeze the Atlas source, human-reviewed reference policy, checker and instrument versions, available tools, model/harness settings, change requests, review rubric, and human-decision budget. Keep the reference policy and result collection outside the implementing agents' control. Compare agents given Warrant's pre-edit context with agents using the existing workflow without that context. Both groups face the same final checker and behavior tests for that stage; M4 also requires the protected deployment and authenticated evidence. Keep retries, unavailable delivery, and human interventions visible rather than comparing only final passing results.

Requests include grouped undo, reuse of an existing owner under different terminology, ambiguous ownership, a legitimate new responsibility, and a change whose combined source invalidates earlier evidence. Separate conformance fixtures test incomplete inventory and rejected forged or stale records; those failures must not be manufactured in a live customer repository.

Record raw counts and denominators, not a composite score:

| Outcome | Measurement and guard against misleading credit |
|---|---|
| Reuse and simplicity | Unnecessary new owners, public interfaces, stores, and wrappers per completed change, judged against the frozen intent. Legitimate new architecture is not penalized. |
| Repair burden | Attempts and rework before the same final acceptance test, including abandoned attempts. |
| Human attention | Questions, interventions, and substantive approval decisions per change; whether the final review was understandable without reconstructing the tool run. |
| Correctness | Required behavior tests and independent review for missed defects, including negative and failure behavior. Passing Warrant alone is insufficient. |
| Retrieval usefulness | Relevant owners and examples found, missed owners, misleading suggestions, ambiguous/no-match accuracy, and how often an agent used the returned interface. |
| Ongoing maintainability | A follow-on change to each result, using the same review rubric, to expose wrappers or coupling that looked harmless initially. |

Reviewers assess results without knowing which context treatment produced them where feasible; disagreements remain visible. Do not infer human agreement from an agent changing code after a warning. Fewer gate failures count as improvement only when policy scope, analysis coverage, and correctness did not weaken. Before the experiment, Trey approves the rubric and what reduction in unnecessary concepts and human intervention would be useful. The release report distinguishes measured benefit, mixed results, and no demonstrated benefit; no positive product claim is made from conformance alone.

### 17.6 Performance budgets

These are requirements measured in CI on the corpus, not estimates of delivery:

- Warm incremental advisory check, after M4 qualification:

  after full-versus-incremental equivalence qualifies the optimization, the check on the pinned Atlas corpus completes in under two seconds wall-clock, excluding external instruments. This is not an M1-to-M3 hook guarantee.
- A cold full evaluation of the same repository completes in under twenty seconds, excluding external instruments.
- Peak resident memory for that evaluation stays under one gibibyte.
- `warrant verify` of a receipt without `--recompute` completes in under one second.

A regression past an applicable budget fails the corpus job. S7 separately measures warm and cold context latency against the supported adapter's deadline. Before safe incremental analysis is available, hooks return bounded current or explicitly stale/unavailable context and provisional diagnostics; a full check exceeding the hook deadline does not become a partial success.

### 17.7 Spikes with exit criteria

Every unproven seam is a spike with a question, a corpus, and an exit criterion, and no milestone that depends on the seam closes before the spike's report lands in `docs/spikes/`.

| Spike | Question | Exit criterion |
|---|---|---|
| S1 (section 6.6) | Which TypeScript 7 surface supplies compiler-authority references in batch: the `unstable/async` sidecar, LSP, or the published SCIP artifact | Precision and recall at or above 0.98 on hand-verified samples, index bytes identical across two clean runs, request count, bytes, and peak memory recorded, every unsupported construct stated; a TypeScript 5 or 6 result never establishes TypeScript 7 authority; D10 records the choice |
| S2 | Retired from Warrant scope | External Git tooling supplies combined source. Warrant verifies and compares the supplied tree; it does not implement or qualify a merge engine. |
| S3 | Can `gix` hash a working tree and an index to the tree `git write-tree` produces | Identical tree ids over the corpus including untracked, modified, ignored, symlinked, and executable files; until it passes, the temporary-index shell-out is the specified path |
| S4 | Does in-process `ssh-key` verification agree with `ssh-keygen -Y verify` | Fixtures signed by `ssh-keygen` across ed25519, ecdsa, and rsa keys verify identically, and the same tampered inputs (bytes, namespace, principal, validity window, revocation) are rejected by both; the fixtures become the conformance suite |
| S5 | Do `RLIMIT_AS` and `RLIMIT_CPU` bind a child spawned from Rust on macOS | Measured; the receipt records `limits: not-applied` with the reason where they do not |
| S6 | Does `oxc_resolver` reach parity with `tsc --traceResolution` on the corpus (section 6.8) | Zero target or outcome disagreements for every enumerated edge in every corpus project, with missing, extra, ambiguous, and unparsed edges counted as failures; a feature-and-mode coverage matrix; each project qualified only for the modes it exercises; any disagreement leaves that project `unqualified` and says so |
| S7 (sections 12.3, 12.6) | Does retrieval find the intended owner, interface, and example under ordinary words, avoid misleading unrelated answers, and reach the model before its decision | A small approved and redacted Atlas request sample plus representative novelty, ambiguity, vocabulary-mismatch, and unrelated-prompt cases; no automatic transcript ingestion or public disclosure. Freeze reviewed labels, failure-cost criteria, and the output budget before held-out measurement. Report found, missed, misleading, ambiguous/no-match outcomes with denominators; returned bytes and omissions; warm/cold latency; and actual delivery timing and failure behavior for every adapter claimed verified. One verified adapter is sufficient for M1. Qualification requires meeting the predeclared criteria, not merely producing a report. Separate development examples from held-out queries; changed criteria require fresh assessment, not a relabeled pass. The report informs whether an optional inferred semantic tier is needed; no embedding service is mandatory. S7 must pass before the R14 outcome comparison. |

## 18. Repository layout and conventions

### 18.1 Layout

Section 3.1's tree, plus:

```
warrant/
  docs/
    vision.md                      the vision; read first
    specs/                         this document and its successors
    design/                        carried-over design notes and the wish list
    research/                      dated research reports; each says what was verified live
    plans/                         plans and live-state.md
    instruments/                   instrument upgrade reports
    acceptance/                    acceptance receipts per milestone
  schemas/                         JSON Schema for every emitted document, generated from Rust types and checked in
  scripts/
    gate.sh                        the full project gate: fmt, clippy, test, conformance, corpus smoke, budget, deny, schema check
    budget.sh                      per-crate source budgets
  tests/
    conformance/
    corpus/manifest.yaml
    acceptance.sh                  the plan's acceptance, never a stub
  warrant/                         Warrant's own policy, rulings, profile, and instruments lock
  AGENTS.md  CLAUDE.md  README.md  LICENSE  Cargo.toml  rust-toolchain.toml
```

### 18.2 Code conventions

- Rust edition 2024; MSRV is the latest stable at the time of each release minus two minor versions, stated in `rust-toolchain.toml` and `Cargo.toml`; CI builds on MSRV and stable.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check` (licenses and advisories), and `cargo test --locked` are the minimum gate.

  `cargo-mutants` on `core` and `authority` per section 17.2.
- No `unsafe` without a `// SAFETY:` comment and a decorrelated review.

  No `unwrap` or `expect` outside tests and `main`. Typed errors with `thiserror` inside crates; `miette` only at the CLI boundary for rendering.
- Functions over traits until there are two implementations.

  TypeScript and the small Cargo implementation justify the language boundary. A disabled advice option is not a second provider implementation and does not justify a provider framework.
- Emitted types generate their schemas.

  Every document uses a `serde` type with a `schemars` derive. CI regenerates `schemas/` and fails on an unreviewed difference.
- Comments say why.

  Names say what. A module's top-of-file comment says what it owns and what it must not know about, in the same words as its `module` contract in `warrant/policy/`.
- Commit subjects at most 72 characters; a soft-wrapped body for anything spanning more than one file; the body states the verification that ran and its count.

  Commit multi-line bodies with `git commit -F <file>`.
- Review regime: `standard` for most changes; `critical` (decorrelated three-model panels looped until no majors) for `crates/authority`, the receipt verifier, the widening classifier, the facet derivation, and every milestone diff.

### 18.3 Warrant's own policy

Warrant checks its crate dependency direction using the minimal Cargo integration, preceded by the ordinary build script during bootstrap. Its policy names each crate and the allowed dependencies from section 3.1. Rust-internal interface or pattern checks are enabled only when a qualified analyzer supplies them. Its required `gate` test receipt may use class `local`, but authoritative acceptance still requires an admitted protected producer (HC19). Bootstrap tests do not claim that Warrant's not-yet-built authoritative gate has passed.

### 18.4 Documentation conventions

Documents are dated in their file names and carry a status line. Research reports say which facts were verified live and when. Prose follows the estate's writing voice (sentence-case headings, no em dashes, no bold lead-ins, straight quotes). The live-state file at `docs/plans/live-state.md` carries what is true now with pointers to the rulings that made it so; it is state, not orders.

## 19. Build-start facts

The versions below are retained from draft 0.1's dated research, not freshly verified by this revision. They are candidate pins, not a claim about today's installed or available releases. M0 rechecks needed packages, compatibility, licensing, and actual runtime identities before locking them. Optional entries are not mandatory dependencies. S1/S4/S6 and the deployment in D5 remain unverified; a version table does not resolve them.

### 19.1 Toolchain

- Rust stable was 1.98.1 on 2026-09-16.

  Edition 2024, which selects Cargo resolver 3 and its MSRV-aware resolution. `rust-version = "1.90"` for `core`, `authority`, `snapshot`, `evidence`, and `cli` (the floor tree-sitter 0.27 sets); `rust-version = "1.96"` for `lang-ts` (the floor oxc sets). The MSRV moves only in minor releases and never above stable minus two; a CI job builds on the pinned MSRV toolchain.
- git 2.40 or newer on PATH (the release that gave `merge-tree --write-tree` its `--merge-base` option); the receipt records the git version that ran.
- OpenSSH 9.1 or newer on any machine that verifies rulings (`-O verify-time` with the `Z` suffix); the signer's `ssh-keygen` is whatever the signer has, and signing needs only 8.1.
- Node, as a check-time sidecar only (R5):

  absent from the oxc fast path, required for the compiler-authority sidecar (S1), for `--traceResolution` parity runs (S6), and for instruments that are npm packages; never resident, never on the fast path.

### 19.2 Crates

| Concern | Crate | Version | License |
|---|---|---|---|
| Git reads, diffs, worktrees, signatures | `gix` | 0.87.1 | MIT OR Apache-2.0 |
| Model store | `rusqlite` (`bundled`, `hooks`, `limits`, `functions`) | 0.40.2 | MIT |
| TypeScript syntax and bindings | `oxc_parser`, `oxc_ast`, `oxc_semantic`, `oxc_span`, `oxc_allocator` | 0.150.0, pinned as one set (the project versions them together and breaks APIs between minors) | MIT |
| TypeScript resolution | `oxc_resolver` | 11.24.3 | MIT |
| TypeScript grammar for patterns | `tree-sitter-typescript` | 0.23.2 (an instrument input, recorded in the lock) | MIT |
| SCIP reader | `scip` | 0.10.0, only if the SCIP candidate survives S1 | Apache-2.0 |
| Rust inventory | `cargo_metadata` | 0.23.1 | MIT |
| Rust syntax | `syn` (full) | 3.0.6, pinned to major 3 | MIT OR Apache-2.0 |
| Pattern contracts | `ast-grep-core`, `ast-grep-config`, `ast-grep-language` | 0.45.3 | MIT |
| Parser runtime | `tree-sitter` | 0.27.0 | MIT |
| Canonical JSON | `serde_json_canonicalizer` | 0.3.2 | MIT |
| Digests | `sha2` | 0.11.0 | MIT OR Apache-2.0 |
| Signature verification | `ssh-key` (`ed25519`, plus `rsa` and `ecdsa` features) | 0.6.7 | Apache-2.0 OR MIT |
| SARIF | `serde-sarif` | 0.8.0 | MIT |
| Output schemas | `schemars` | 1.2.2 | MIT |
| Graphs | `petgraph` | 0.8.3 | MIT OR Apache-2.0 |
| CLI | `clap` | 4.6.7 | MIT OR Apache-2.0 |
| Diagnostics | `miette`, `thiserror` | 7.6.0, 2.0.20 | Apache-2.0; MIT OR Apache-2.0 |
| JSON | `serde`, `serde_json` | 1.0.229, 1.0.151 | MIT OR Apache-2.0 |
| YAML | `serde-saphyr` | 1.3.0 | MIT OR Apache-2.0 |
| Walking and globs | `ignore`, `globset` | 0.4.33, 0.4.20 | Unlicense OR MIT |
| Logging and progress | `tracing`, `tracing-subscriber`, `indicatif` | 0.1.44, 0.3.23, 0.18.6 | MIT |
| Cancellation and parallelism | `ctrlc`, `rayon` | 3.5.2, 1.12.0 | MIT OR Apache-2.0 |
| Child processes and limits | `nix`, `process-wrap`, `wait-timeout` | 0.31.3, 10.0.0, 0.2.1 | MIT; Apache-2.0 OR MIT; MIT OR Apache-2.0 |
| Time | `jiff` | 0.2.37 | Unlicense OR MIT |
| Tests | `insta`, `tempfile`, `pretty_assertions` | 1.48.0, 3.27.0, 1.x | Apache-2.0; MIT OR Apache-2.0; MIT OR Apache-2.0 |

Choices inside the table that are mine rather than the report's: `jiff` over `chrono` (RFC 3339 and civil time without a separate timezone crate; the downside is a younger ecosystem, and nothing in Warrant needs chrono's integrations); `serde_json_canonicalizer` over `serde_jcs` (the report's pick; both implement RFC 8785, and the conformance suite carries the RFC's own test vectors so a swap is a one-line change). Not taken, with reasons in the report: `git2` (C build, 2026 advisories, no merge-tree), `sqlx` (a server-shaped API for a file), `serde_yaml` and `serde_yml` (both archived), `ra_ap_*` crates (explicitly unstable; rust-analyzer is a process instead), rustdoc JSON (nightly only).

### 19.3 External tools Warrant runs

- `git` for object reads and source comparison, and temporary-index source capture until S3 qualifies the native path. Combining branches remains an external Git operation.
- `ssh-keygen`, for signing only, on the signer's machine, and as the conformance oracle for verification.
- TypeScript's compiler:

  `typescript@7.0.2`, the stable native compiler (it installs `tsc`; the executable comes from the platform package the main package selects, which the receipt records with its binary digest), as the parity oracle (`--traceResolution`), the diagnostics source, and, per spike S1, the compiler-authority sidecar through its `unstable/async` client. `typescript@6.0.3` is the last JavaScript-based compiler; `@typescript/typescript6@6.0.2` is the compatibility wrapper that resolves it, used only inside S1's source-pinned SCIP candidate; `typescript@5.9.3` is recorded only if a 5.x instrument ever runs. `@typescript/native-preview` (latest `7.0.0-dev.20260707.2` at research time) is a development build and is not selected. `@sourcegraph/scip-typescript@0.4.0` is an S1 candidate only; its published artifact resolves `typescript@^5.6.2`.
- `rust-analyzer` is a possible later Rust-reference source, not an initial dependency. Its absence is not missing required evidence unless a future approved contract explicitly requires that instrument.

### 19.4 Candidate evidence instruments

`typescript@7.0.2` (diagnostics and trace adapters), `knip@6.36.0`, `fallow@3.27.0` (exit 1 means findings, not failure), `dependency-cruiser@18.3.1` (census importer only), `@stryker-mutator/core@10.0.0` with `mutation-testing-report-schema@3.8.4` and `mutation-testing-metrics@3.8.4` as the adapter's schema and metric identity, `vitest@5.0.1`, and `jest@30.5.1`. No formal JSON Schema was verified for knip, vitest, or jest; their adapters' fixtures are the format contract. Every entry in `warrant/instruments.lock` carries name, version, registry integrity, dependency-lock digest, platform binary digest where one exists, adapter version, and the arguments Warrant passes; section 9.4 has the format.

### 19.5 Judgment backend facts

TypeSafe's System One API: `POST https://api.typesafe.ai/v1/systemone` with bearer authentication; OpenAPI version 0.2.0; request fields `state`, `model`, `questions`; the response's `model` string may differ from the alias sent and is what the receipt records; `jev-latest` is the flagship alias and `jev-1.12` is the id the published cookbooks pin; official SDKs at 0.6.0 (JavaScript and Python), whose 0.6.0 made `score` criteria an ordered sequence; budget about 32,000 tokens shared by state and questions with no published tokenizer; `choice` at most 255 options; errors 401 (observed as 403 without a key), 422, 429, 529; access is by waitlist. All as of 2026-09-16 from `docs/research/2026-09-16-typesafe-jev.md`, which also records the price at research time and the reasons not to design for it holding.

### 19.6 Estate facts

- First customer: Atlas at a pinned commit [set at M0], whose architecture spec's hard constraints translate to the worked policy in section 7.8.
- Signer and gate:

  the human ruling key remains outside agent reach. The authoritative gate and evidence-runner deployment are not yet verified. D5 requires separation of candidate execution from protected acceptance controls; the draft 0.1 same-user gate proposal is withdrawn.
- Repository: Forgejo is origin; GitHub `treygoff24/warrant` is the public mirror. License per R10. No internal addresses or machine paths belong in published artifacts.

## 20. Decisions

### 20.1 Ruled

- R1 (Trey, 2026-09-16): fresh repository, not an evolution of Specgate; no consumers to preserve.
- R2 (Trey, 2026-09-16): the name is Warrant.
- R3 (Trey, 2026-09-16): reuse existing excellent tools wherever possible; novel contributions are limited to what section 3.4 enumerates.
- R4 (Trey, 2026-09-16): rulings are signed by Trey with a key agents never see; agents draft and never ratify.
- R5 (Trey, 2026-09-16):

  Rust first is a goal; other languages are fine where they are the source of truth; a Node process at check time is acceptable if it is not resident.
- R6 (Trey, 2026-09-16): a frozen corpus including Atlas and dependency trees is the test bed for instrument and tool upgrades.
- R7 (Trey, 2026-09-16): the widening classifier is ours to own.
- R8 (Trey, 2026-09-16): TypeSafe's System One shape has a home in Warrant as the judgment lane's interface, with the line that judgments never enter compliance.
- R9 (Trey, 2026-09-16): public open-source repository on GitHub from day one, with Forgejo as origin per the estate rule.
- R10 (Fable, 2026-09-16, recorded as a ruling because Trey delegated the repository setup):

  the license is Apache-2.0 OR MIT dual, the Rust ecosystem's convention. Downside: a permissive license forecloses nothing commercially but also reserves nothing; Trey can change it before the first outside contribution with no cost.
- R11 (Trey, 2026-09-17, review conversation):

  Warrant does nothing that even appears to be task, planning, or project management. Existing planning skills, Beads, and agent workflows own that work. Architectural context and concrete-source checking remain Warrant's job.
- R12 (Trey, 2026-09-17, reuse clarification):

  prefer existing tools when they work well for the need and reduce total complexity, including connection, verification, and maintenance. Use a narrower custom piece when adapting the existing one would be more complex. This does not authorize blanket removal of integrations.
- R13 (Trey, 2026-09-17, "good, approved, go"):

  revise the spec around the agreed review direction, obtain Fable review, and return a plain-language summary plus remaining decisions. This authorizes the revision, not ratification, an implementation plan, or a build.
- R14 (Trey, 2026-09-17, co-design review):

  test whether the architectural guidance improves real agent work before building the protected acceptance system. If the early advisory comparison shows no benefit, stop and rethink the guidance rather than automatically continue to the full gate. This is a build-sequence checkpoint, not a work-management feature inside Warrant.
- R15 (Trey, 2026-09-17, co-design review):

  v1 has two human approval actions: amend the rules, or accept a specific known violation. Instrument adoption and accepted limitations are typed amendments; trust-root administration stays outside Warrant and team delegation is deferred. An emergency bypass never turns an unverified result into a pass.
- R16 (Trey, 2026-09-17, co-design review):

  measurable preferences give visible advice by default. Blocking or required human approval is an explicit per-rule choice; architectural invariants retain their blocking behavior.

### 20.2 Decisions for ratification and build readiness

Entries marked ruled cite Trey's decision; the others remain recommendations or evidence prerequisites for ratification and build readiness. D3 corrects the previous contradiction between "never blocks" and `approval: required`. D5 rejects the earlier same-user gate arrangement. R14-R16 settle the early checkpoint, two-action approval design, and preference default, not the untested technical claims.

- D1. Policy directory: retain visible `warrant/`. Naming is reversible before release.
- D2. Permanent identifiers for signed records:

  use an owned domain if Trey already controls or chooses to acquire one; otherwise use the repository's GitHub-based identifiers. No purchase is authorized. Resolve before issuing durable signed records; examples using `warrant.dev` are placeholders. D13 and D14 determine the exported forms first.
- D3. Model opinions and acceptance:

  recommend advisory-only for the first product. Keep the typed interface, but defer model-triggered required review. If later enabled by a signed policy, it explicitly blocks acceptance and must specify outage behavior and measured usefulness. This replaces the old "never blocks, only requires approval" formulation.
- D4. Corpus publication:

  recommend private storage for Atlas/dependency artifacts plus a public synthetic corpus and manifest containing only publishable metadata. Do not publish Atlas tarballs or dependency bundles without checking source permissions and data exposure. Hosting choice is secondary to that boundary; GitHub publication remains separately gated.
- D5. Authoritative execution:

  qualify the protected-controller and credential-free worker candidate in section 3.7 using an existing runner, with protected Warrant installation, approved-policy anchor, producer identity, and receipt signer. Candidate tests have none of those credentials or write access. R14's useful-guidance checkpoint precedes building this deployment; actual account, service, and infrastructure changes need separate approval. Verify the selected Atlas arrangement before M3 closes. A read-only mount or separately owned trust-root file alone does not qualify. No hosting or infrastructure change is authorized by this spec revision.
- D6. Self-check bootstrap:

  use the ordinary Rust build/test gate and a small Cargo dependency-direction check at M0. Replace the dependency check with the minimal Cargo integration when available. Warrant cannot authoritatively check itself before its verifier and acceptance controls exist.
- D7. Mutation thresholds outside core and authority: report first, then choose per crate from observed test quality. Core and authority retain section 17.2's hard-control requirements.
- D8. npm installation wrapper: consider after the first release if TypeScript users need it; use existing packaging rather than a Warrant package manager.
- D9. Telemetry: recommend no telemetry code path. Local experiment artifacts and voluntary user reports answer product questions.
- D10. S1 compiler-reference decision: evidence-dependent, concluded before promising any contract that needs it. A failed spike means an honest unsupported claim or narrower product requirement, not a relabeled pass.
- D11. Deterministic preferences:

  ruled by R16: visible, counted advice is the default. Explicit `fail` or `review` is a signed per-rule choice. Invariants and migrations retain blocking defaults; model advice follows D3 separately.
- D12. Local config path: retain the conventional Warrant config directory. Its location does not establish trust; protection and admission follow D5.
- D13. VSA export: retain the documented mapping and its conformance check; implement the export command when a consumer needs it. The full four-fact receipt stays primary.
- D14. Signature carrier: retain detached SSH signatures for the initial product. Defer DSSE or provider-specific carriers until needed; preserve record bytes and a small verification boundary without building unused implementations.

S1, S7, the R14 outcome result, and deployment verification in D5 are evidence conditions, not facts a human can establish by preference. The final ratification records which remaining defaults Trey accepts and which stay held; unresolved release prerequisites cannot be marked complete by an implementation plan.

## 21. Milestones

This is dependency order for building Warrant, not a task-management feature. The subsequent `writing-plans` skill turns the ratified spec into build work. Each milestone proves the named behavior with schemas, conformance fixtures, and relevant frozen-corpus checks. No build is authorized by this draft.

| Milestone | Scope | Observable exit and limits |
|---|---|---|
| M0. Exact inputs and a testable foundation | Workspace and ordinary Rust gate; snapshot capture/read for worktree, index, commit, and supplied tree; inventory with independent classification and ownership; conformance fixture harness; Atlas revision and actual grouped-undo references selected; dependency and instrument inputs recorded. | Staged and committed reads ignore unrelated worktree bytes; tracked ignored files remain present; missing/unread input is explicit; colocated tests retain their class. S3 resolves native capture or keeps the Git path. This proves source identity and inventory, not architectural acceptance. |
| M1. A useful map before decisions | TypeScript model, S6 parity, declaration schema and canonical policy definitions, query, manifest-free census and starter proposals, context, single-change propose, Mermaid/JSON map; small Cargo package-dependency integration; one before-decision harness adapter. Resolve S1 and qualify retrieval/delivery through S7. | Ordinary-language queries return real references, reuse candidates, examples, consumers, and required evidence under S7's predeclared criteria. Ambiguous, unrelated, no-match, incomplete, stale, and legitimate-new-owner cases are distinct. A recorded supported-harness run proves context preceded the decision, including nondelivery cases. Qualified resolution has zero disagreements on covered features. Cargo exercises the language boundary without full Rust internals. No capability exceeds its measured scope. |
| M2. Honest advisory checks and the early product checkpoint | Typed claims, dependency/interface/pattern and declared ownership checks, capability structure and evidence requirements; four-fact advisory verdict; visible preference advice and explicit stricter choices; signals and explanations; two ruling kinds with exact exception batches; human signing and policy-anchor chain; S4 verification. Then run section 17.5's early advisory comparison. | Stories A-C distinguish reuse suggestions, signed policy changes, and incomplete analysis. New source under unchanged rules keeps policy identity. Present-day obligation equality does not prove future equivalence; call occurrence does not prove authorization behavior. Record onboarding exception burden and policy-change classifications. R14 then requires useful-guidance evidence: no demonstrated benefit stops progression for reconsideration, and mixed results go to Trey. This does not prove deployed runner isolation. |
| M3. The first complete Atlas experience | Only after the R14 checkpoint permits progression: D5's separately authorized and qualified protected deployment; exact-source execution; needed evidence adapters selected by Atlas's requirements; approved contracts and exceptions; full authenticated gate receipt; fresh combined-source evidence; human paragraph. | Run all of section 1.7. Selected bytes, admitted producers, actual required cases, service/input bindings, and the protected comparison base govern acceptance. Other-tree receipts remain stale. Story D compares supplied revisions with honest attribution and no lane registry. All four facts bind to the exact candidate and approved configuration. Infrastructure proof and human policy signatures are real prerequisites, not fixture substitutes. |
| M4. Independent value evidence and dependable upgrades | Section 17.5's held-out complete-product comparison, separate from tuned pilot requests; instrument/self qualification and upgrade reports; installed-control self-test; finding comparisons; safe incremental analysis after equivalence qualification. | Stories E and F distinguish simpler code from lower counts and instrument changes from code improvement. Changed inputs, trust, or time cannot inherit stale acceptance. Report held-out outcomes and follow-on maintainability with limits; revise guidance if benefit is not demonstrated. The warm incremental performance budget applies only after the optimization qualifies. |
| M5. Fit the existing workflow and release | Thin MCP and additional needed harness/diagnostic adapters beyond the one proven at M1; complete CLI/schema docs and bounded output; optional typed advice if useful; first-adopter importers only where needed. | CLI and supported transports preserve evidence and delivery labels. Diagnostics never look like clean checks; model outages change no acceptance facet; no task-management surface appears. Required repository gates and a fresh installed-runtime run pass on the release candidate. Unused frameworks, languages, formats, and providers remain outside release scope. |

M1 and M2 provide useful advisory capabilities and test the upstream thesis before the protected system is built. The first complete map-plus-authoritative-check product is M3; M4 supplies independent complete-product benefit evidence, and M5 prepares the supported workflow for release. Full Rust source references, extra diagram formats, unused policy importers/report readers, cross-tree test-evidence carry-over, team delegation, and a multi-provider judgment platform are not hidden release obligations.

Port only useful predecessor code: parser/resolver/discovery pieces behind the new integration contract, fixture projects as conformance seeds, and fail-closed policy-comparison cases. Do not carry forward the baseline, file-only model, old verdict, or accumulated doctor/config surface merely because they already exist.

## 22. Risks

- The map cannot find an existing owner under the user's words.

  S7 tests vocabulary mismatch, aliases, ambiguity, misleading answers, and omissions before M1 closes. R14 tests actual outcomes before the protected gate is built; no benefit means stop and rethink rather than invest automatically in enforcement.
- Context is delivered too late or not at all.

  Verify model-input timing, installed hook trust, timeout/skip behavior, and actual harness coverage. A configured pre-tool hook is not proof that an agent could reconsider its pending edit.
- Available TypeScript references do not support a promised claim. Resolve S1 early, declare required evidence capabilities, and retain `enforcement-unsupported` rather than weakening meaning in prose.
- The protected gate is only nominally protected.

  D5 must verify executable/configuration control, exact execution source, evidence-producer authority, and candidate isolation using the real deployment before M3 closes. No local fixture substitutes for that evidence.
- Policy comparison asks for too many signatures.

  Keep canonical rules independent of changing source subjects; prove a small set of grammar transformations; leave uncertain rule changes explicit. Do not infer equivalence from one snapshot to reduce friction.
- Signing becomes routine rubber-stamping. Avoid model-driven review by default, present concise rule effects, and measure substantive human interventions. A recorded render digest cannot prove the signer understood it.
- Borrowed tools add more complexity than they remove.

  Judge each integration by needed behavior, adaptation cost, verification, and maintenance. Keep raw evidence and fixture contracts so changing a provider does not change meaning silently.
- Fresh combined-source evidence costs more than cross-tree reuse. Accept the cost initially. Do not optimize until the complete execution-input boundary is known and independently testable.
- Model advice is misleading or sends more source than intended.

  Keep it optional, scoped, labeled, and advisory; measure benefit before expanding it. Hosted service access and calibration remain unproven by this revision.
- Corpus or report publication exposes private source or data. Use synthetic/public artifacts by default; obtain separate approval before publishing real Atlas artifacts.
- More machinery substitutes for a better product.

  Keep section 1.7 as the complete experience and section 17.5 as its outcome test. Added command families must serve that experience, not an abstract platform ambition.
- Users mistake the in-toto Statement shape for compatibility with every signing tool.

  Document that the initial detached SSH carrier needs Warrant or SSH verification; a future carrier is a separate integration.

## 23. Decision log: considered and rejected

The per-section rejections are collected here in one place so a reviewer can argue with the whole set.

### 23.1 Evolving Specgate instead of a fresh repository

Rejected because five properties of the predecessor are each the negation of a vision idea and each is load-bearing in its code: a file-level, import-centric model (the vocabulary is `boundary`, `dependencies`, `layers`, `unique_export`, not modules and effects); parse failures, unclaimed files, and dynamic imports as warnings (story C is built in); the baseline as a green snapshot (HC7's opposite); lexicographic canonical-config precedence (W06's opposite); and a command surface of accreted `doctor` subcommands and configuration fields with no runtime consumer (Astra's Atlas research found `escape_hatches.*` and inline-ignore expiry have none). Roughly eighty percent of the code is left behind; the parser, resolver, discovery, fixtures, and the classifier's fail-closed rules come along as seeds.

### 23.2 Alternatives rejected per section

- Snapshots: own hashing; mtime; descending submodules (4.6).
- Inventory: first-match-wins; unknown-as-source; framework knowledge in the core (5.8).
- Model: in-memory only; compiler-only; tree-sitter for TypeScript; name-based inference of registrations; a graph database; `unstable/*` as stable; the published SCIP artifact as TypeScript 7 authority (6.10).
- Policy: Rego; Cedar as the language; a code DSL; built-in layer taxonomies; severity levels (7.9).
- Findings: severities; autofix in v1; positional identity (8.7).
- Evidence: running tests; boolean "passed"; normalized meaning; auto-updating instruments; exit status as the finding set (9.6).
- Verdict: single pass or fail; warnings; per-facet exit codes; a bespoke receipt; a VSA as the receipt; model-checked receipts (10.7).
- Authority:

  baseline owner fields; signed commits as approval; GPG; Sigstore keyless in v1; a user database; one namespace for all kinds; DSSE as the v1 default; RFC 3161; drafter-set `signed_at` (11.8).
- Interfaces: `doctor` families; two CLIs; embedded source; LSP in v1 (12.7).
- Lanes: Warrant as orchestrator; a global lock; path-based invalidation (13.7).
- Map: C4 and Structurizr models; a hosted viewer (14).
- Census: accepting permissive proposals; directories as law; clustering for boundaries (15.5).
- Judgment: paragraph reviews; judgments in compliance; composite scores; Jev as required; whole-file states; intent matching as a `noul`; stable confidence; recomputed judgments (16.7).

### 23.3 Two things I nearly did and did not

- A fifth facet for judgment.

  Optional model advice remains separate from the four acceptance facts. Required model-triggered review is a possible later policy choice, not something made harmless by calling it approval.
- A general policy-analysis language inside the first classifier.

  Initial comparison uses tested transformations of the canonical rule grammar and returns uncertain elsewhere. Comparing today's resolved obligations is useful impact information, not proof of policy equivalence. A broader analyzer is justified only if that limitation creates a demonstrated problem.

## 24. Traceability

This table describes intended scope, not implemented capability. There is no Warrant implementation yet. "Specified" means this draft requires it; a future report must supply the evidence before calling it delivered.

| Wish | Section | Scope in the revised draft |
|---|---|---|
| W01 inventory and entrypoints | 4, 5 | Specified with explicit omissions and independent classification/ownership |
| W02 resolve the same program | 6 | Partial: qualified module resolution and binding references; stronger references depend on S1 |
| W03 architectural questions | 6.9, 12.3, 12.6, S7 | Specified early, including retrieval failures and verified before-decision delivery |
| W04 across languages | 6.1, 6.7 | TypeScript plus a minimal Cargo dependency implementation; full Rust internals and other languages deferred |
| W05 domain contracts | 7 | Specified; typed checked claims distinct from broad intent |
| W06 unambiguous policy | 7.6, 11.5 | Specified; canonical rules separate from snapshot obligations |
| W07 effects and sensitive data | 7.4, 9 | Partial: observed adapter consumers and recognized sites; no universal route, authorization, or data-flow proof; no sensitivity removal by naming a summarizer |
| W08 connect a capability | 7.4, 9 | Partial: structure plus required authenticated behavior scenarios; not proof of all runtime behavior |
| W09 paper architecture | 7.6 | Drift checks specified with evidence limits |
| W10 existing responsibilities | 12.3, 12.4 | Reuse candidates and examples specified; inferred duplication labeled |
| W11 conceptual expansion | 8.5 | Specific signals; legitimate new architecture remains possible |
| W12 borrow detectors | 9 | Needed external evidence and shared formats retained; unneeded readers deferred |
| W13 metric gaming | 8.5, 17.5 | Signals plus scope/denominator checks; counts alone do not establish improvement |
| W14 material relaxation | 11.5, 11.6 | Supported proof rules; uncertain changes require approval; no snapshot-equivalence shortcut |
| W15 edit versus approve | 3.7, 7.6, 11 | All active rule replacements require verified human authority |
| W16 exceptions | 11.4 | Scoped, signed dispositions with count and validity bounds |
| W17 incomplete is not success | 5.5, 8.2, 10 | Four facets and distinct check/diagnostic/gate outputs |
| W18 composable CLI | 12.1, 12.2 | Architectural/evidence surface only; no work-management commands |
| W19 meaningful causes | 8.3, 8.4 | Findings, consumers, explanations, and focused checks |
| W20 proposed change | 12.4 | One advisory architectural query; source-bound patch preview; no task state |
| W21 harness integration | 12.5, 12.6 | Needed MCP/hooks after useful CLI; LSP deferred |
| W22 patch versus combined product | 13 | Source comparisons and fresh combined evidence retained; lane registry and work overlap excluded |
| W23 explainable invalidation | 3.5, 13.3, 13.4 | Safe analysis caching; cross-tree execution-evidence reuse deferred |
| W24 meaningful receipts | 9.2, 10, 11 | Protected producer, exact source and execution inputs, independent verification |
| W25 predictable environments | 3.6, 12.2 | Bounded offline core and explicit optional network advice |
| W26 onboarding | 15 | Minimal census early; intent ratified separately; imports only when needed |
| W27 findings into bounded work | 8.3, 15.4 | Work management excluded; findings retain evidence and remedy explanations for external tools |
| W28 intent without false authority | 7, 11, 16 | Signed intent; model advice never supplies authority |
| W29 calibration and usefulness | 17.5, 16.6 | External outcome comparison; provider-calibration platform deferred |
| W30 conformance | 17.1, 17.2 | Specified, including false-assurance and execution-boundary cases |
| W31 instrument upgrades | 9.5, 11.6 | Corpus comparison plus explicit adoption; equal samples are not universal equivalence |
| W32 installed self-test | 17.4 | Covered controls tested with uncovered ones explicitly reported |
| Story A second implementation | 12.4, M1/M2 | Existing owner shown without mistaking resemblance for a proven defect |
| Story B weakened policy | 11.5, M2 | Candidate cannot approve its own rule replacement |
| Story C skipped source | 5.5, 10.1, M0/M2 | Missing scope remains visible and blocks full acceptance |
| Story D clean branches, broken combination | 13, M3 | Exact supplied trees and fresh evidence; no coordination state |
| Story E misleading cleanup | 8.5, 17.5, M4 | Review actual responsibility/coupling changes, not only counts |
| Story F instrument change | 9.5, 11.6, M4 | Separate instrument adoption from code improvement |

## 25. References

- `docs/vision.md`: the vision, read first.
- `docs/design/2026-09-16-agent-builder-wishlist.md`: the requirements brief (Codex, 2026-09-16).
- `docs/research/2026-09-16-typesafe-jev.md`: TypeSafe System One and Jev; the judgment lane's interface.
- `docs/research/2026-09-16-typescript-analysis-stack.md`: TypeScript 7, oxc, resolution, symbol sources, evidence producers.
- `docs/research/2026-09-16-rust-building-blocks.md`: crates and system tools, with pins.
- `docs/research/2026-09-16-signing-and-attestation.md`: sshsig, allowed signers, in-toto, canonical JSON, threat model.
- `docs/research/2026-09-16-prior-art.md`: eight families of prior art, what Warrant borrows, what it rejects, and the gaps no surveyed tool fills.
- `docs/research/2026-09-16-polyglot-integrations.md`: the integration contract across languages.
- `docs/design/atlas-quality-research/`: Astra's Atlas quality-tooling notes (2026-09-16), copied with provenance.
- The predecessor: Specgate v0.3.2 (`treygoff24/specgate`), whose parser, resolver, discovery, and fixtures seed M1 and the conformance suite.
