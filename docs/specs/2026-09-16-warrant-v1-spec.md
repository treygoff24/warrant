# Warrant v1 specification

Version: 0.1 draft, 2026-09-16. Status: draft for review by Trey, Astra, and Fable; nothing here is ratified. Owners: Trey Goff, Fable, and Astra. Warrant is co-owned by the three of us; this draft was written by Fable from Trey's direction and the Codex wish list, and Astra's review comes next.

## Read the vision first

If you are working on this spec, or building from it, read `docs/vision.md` in this repository first and keep it open. The spec exists to make the vision buildable; when the two disagree, the vision's four ideas win and this document gets fixed.

The vision in three sentences, for the reader who will not click: AI slop is a contract problem. Agents write slop because they are guessing about where behavior belongs, what owns it, which interface to use, and what proof will be required, and because when a check fails the agent holds the pen for the rules too. Warrant fixes that upstream with three moves: a map before the edit, a contract during, and a receipt after, with the rules signed by a human key the agents cannot reach.

Companion documents in this repository:

- `docs/design/2026-09-16-agent-builder-wishlist.md`: the requirements brief, thirty-two wishes (W01 to W32) and six acceptance stories (A to F), written by Codex as an agent that builds Trey's software. Section 24 of this spec maps every wish and story to the section that delivers it.
- `docs/research/`: the research reports behind the borrowed pieces. Each is dated and says what was verified live.
- `docs/design/`: the design notes carried over from Specgate, Warrant's predecessor, including the Atlas quality-tooling research Astra wrote on 2026-09-16.

## 0. How this spec was written: the decision criteria

These are the heuristics I used to decide what goes in this document and how each decision was made. They are mine, written down so that Trey and Astra can argue with the method and not only the results, and so that when we change our minds later we can say which heuristic was wrong.

1. The vision governs, the wish list specifies, research informs, and taste breaks ties, in that order. When a wish conflicted with one of the vision's four ideas, the idea won and the wish was scoped down with a note. When research showed a borrowed piece would not do the job, I changed the design, not the research.
2. Borrow before build. Every seam names the tool that already does the work and records why the alternatives lost. The code we write ourselves is enumerated in section 3.4 and nowhere else. A component that is not on that list and not borrowed is a mistake in this spec.
3. Fail closed wherever a verdict depends on it. "Could not analyze" is a failure of the analysis facet, never a warning that decays into a pass. Fail-open behavior exists only where a person chose it, in a profile, in writing.
4. Deterministic core, probabilistic rim. Nothing a model produced is ever an input to the compliance facet. Model output is labeled as judgment, carries its probability, and routes to a human decision or to advice. This is the line the wish list draws in W28 and the vision draws in "What it refuses to be," and every section of this spec that mentions a model repeats it.
5. Fewest nouns. A concept enters the vocabulary only if it appears in a verdict, a receipt, a query, or a policy file. Seven nouns carry the product (section 2). Where I wanted an eighth I looked for which of the seven it really was.
6. Agents are the primary user and the structured output is the API. Human-readable output is a rendering of the JSON, never a separate truth. A feature an agent cannot drive from the CLI or the MCP surface does not exist.
7. Cheap to verify beats cheap to produce. A receipt is machine-verifiable; a ruling is signed; a hard control has a fixture that fails when the control is broken. Where verification would be expensive, I chose the design that makes it cheap over the design that makes production cheap.
8. Reversible defaults now, irreversible choices only when a fixture forces them. Formats, identifiers, and directory names are chosen so that changing them later is a rename; where a choice would be hard to reverse (the receipt predicate type, the signing namespace, finding identity), it is listed for Trey to rule on in section 20.
9. TypeScript first, language-neutral by construction. The core never knows what a TypeScript import is. The first language integration proves the integration contract; the second (Rust, so Warrant can check itself) proves that the contract was not secretly TypeScript-shaped.
10. Rust first as a goal, not a rule. Where the source of truth is a Node program (the TypeScript compiler) or a Python one, a sidecar process is acceptable when it runs at check time and exits. No long-lived daemon is required to use Warrant.
11. Everything unproven is a spike with an exit criterion. Symbol-level references, the judgment lane's accuracy on code, and the multi-lane invalidation model are each marked as spikes with the question they must answer, and the milestone that depends on them names the fallback.
12. Say where v1 under-delivers on purpose. Section 24 marks each wish as delivered, partial, or deferred, with the reason. A partial that is not marked is a bug in the spec.
13. Write for the compiler. This spec will be compiled into a plan and a bead graph by the `writing-plans` workflow. Every requirement is numbered, testable, and phrased so a lane can close it; every milestone exit criterion says what green proves and what it does not.
14. Dogfood on Atlas, and then on Warrant. Atlas is the first customer and the frozen corpus's first real repository; Warrant checks its own repository as soon as the Rust integration exists. A rule we would not accept on our own code does not ship.
15. No time estimates anywhere in this document. Order and dependencies, yes. Durations, no.
16. Ownership is shared. Where I made a call that Trey or Astra might make differently, I wrote the alternatives and the reason I chose as I did, so that overruling me is cheap and recorded.

### 0.1 What I read and what I ruled out before writing

I read the whole predecessor (Specgate v0.3.2: roughly 7,000 lines of Rust across parser, resolver, graph, rules, verdict, baseline, and policy diff, plus 841 tests and fourteen fixture projects), the wish list, the vision, Astra's four Atlas quality-tooling notes and the TypeScript 7 architecture note, and TypeSafe AI's announcement and documentation for their System One model class. Six research lanes verified the borrowed pieces live on 2026-09-16; their reports are in `docs/research/` and this spec cites them by file name.

I ruled out evolving Specgate in place before writing a line of this document. The reasons are in section 23.1; the short version is that the predecessor's data model is file-level and import-centric, its baseline is a green snapshot, its policy precedence is lexicographic, its parse failures are warnings, and its command surface grew by accretion. Each of those is the opposite of a vision idea. What survives is the parser and resolver code as a starting point for the TypeScript integration, the fixture corpus as the seed of the conformance suite, and the policy-diff classifier's fail-closed rules as the seed of the widening classifier.

## 1. Product summary and scope

### 1.1 One paragraph

Warrant is a command-line tool, written in Rust, that reads a repository at an exact content-addressed snapshot, builds an inventory of every file with a classification and a reason, builds a program model from the language's own resolution rules, evaluates a set of human-signed contracts against that model, and produces a verdict with four separate facts (compliance, analysis completeness, evidence, approval) bound to a machine-verifiable receipt. Agents query the model before they edit, check their lane while they work, and submit an integrated candidate to a gate whose exit zero means every obligation was met by real evidence or a valid signed exception. Humans sign the rules and the rulings with an SSH key that agents do not hold. Optional model judgment lives in a swappable profile and can only ever route a change to a human, never pass or fail it.

### 1.2 Goals for v1

- G1. An agent can ask Warrant where a responsibility lives, what owns it, which interface to use, which callers depend on a symbol, and why a dependency is or is not permitted, and get an answer with exact paths, symbols, and a statement of which parts are declaration, static observation, or inference (W03).
- G2. A repository's architecture can be declared as contracts in the domain's vocabulary (ownership, interfaces, dependency direction, effects and their permitted callers, state ownership, capability links), each carrying intent, owner, authority, enforcement mode, and stated limits (W05, W07, W08).
- G3. Every verdict carries four facts that cannot collapse into one, and every integration-gate pass is backed by a receipt a machine can verify (W17, W24).
- G4. Rules and rulings are signed records; an agent can draft one and cannot ratify one; every policy change is classified as narrowing, widening, restructuring, or uncertain, and uncertain is treated as widening (W14, W15, W16).
- G5. The inventory and the program model are complete by construction: a file that could not be read or a construct that could not be analyzed appears as missing evidence, and the denominator is always visible (W01, W02).
- G6. Two lanes that each pass can produce an integrated candidate that fails, and the failure is attributed to the combination, with each lane's evidence still valid for its own snapshot (W22, W23).
- G7. An existing repository can be onboarded by a census that proposes contracts from observation and keeps observed and intended architecture distinct (W26).
- G8. Taste is a profile: measurable preferences enter as evidence obligations, judgment preferences enter as labeled model questions, and both live in a file a user can replace (vision, "Taste is a profile").
- G9. Warrant proves its own claims: every hard control has a conformance fixture, the gate is mutation-tested, and instrument upgrades produce a before-and-after report on a frozen corpus before adoption (W30, W31, W32).

### 1.3 Non-goals

For v1 and, unless a ruling changes it, forever:

- Not a linter, formatter, test runner, package manager, workflow engine, task ledger, or account system. Those tools exist and Warrant consumes their output or exports to them (W12, "What I do not want").
- Not an AI judge. No model output is the source of a hard rule, a compliance result, or an approval (W28).
- Not a score. There is no single number that summarizes a codebase or a change. There are contracts, evidence, findings, and unresolved obligations (W13, vision).
- Not a hosted service. Core operations run offline on the developer's machine or in CI with no network dependency (W25). The judgment lane's remote backends are optional and explicitly enabled.
- Not an architecture designer. The census proposes; a human ratifies (W26).
- Not a taint analyzer in v1. Sensitive-data contracts are declarations plus pattern recognition with stated limits, never a claim of proven data flow (W07).

Deferred from v1 with a path, not refused:

- Editor diagnostics over LSP (W21). The finding format is designed to render as LSP diagnostics; the server is a v1.x deliverable.
- Compiler-level symbol references for TypeScript (W02). v1 ships a binding-level symbol graph from oxc; the compiler-authority path is spike S1 (section 6.6).
- Languages beyond TypeScript and Rust (W04). The integration contract is designed in v1 and proved by two implementations; Python and SQL follow it.

### 1.4 Users

- Coding agents (Claude Code, Codex, Cursor, and any harness that can run a CLI or speak MCP). They read the map, run lane checks, submit candidates, and draft rulings.
- The signer. A human with an SSH key on a machine the agents do not control. In this estate that is Trey, signing as the `trey` user on the devbox, while agents run as `trey-agent`.
- The gate. A CI job or coordinator that runs `warrant gate` on the integrated candidate with a trust root it controls.
- Reviewers. Humans and models who read explanations and receipts. Reviewers do not need Warrant installed to verify a receipt; `warrant verify` is a separate, dependency-light operation.

### 1.5 Hard constraints, binding on every change to Warrant

- HC1. No probabilistic input to compliance. The compliance facet is computed from the program model, the effective policy, and deterministic evidence only. A judgment result can set `approval: required` or attach advice; it cannot change `compliance`.
- HC2. Exit zero on `warrant gate` means every applicable obligation was satisfied by evidence bound to this snapshot or by a valid signed exception, the analysis was complete, and no approval is pending. Any other state exits nonzero.
- HC3. Unread is not absent. A file the inventory could not read, a construct an integration could not analyze, a report that was truncated, or a required instrument that was missing sets `analysis: incomplete` and is listed with its reason.
- HC4. Every verdict has a receipt. A verdict without a receipt is not a verdict; the receipt binds the snapshot digest, the effective-policy digest, the inventory digest, every instrument identity, every external evidence digest, and the evaluation clock.
- HC5. Trust roots live outside the candidate. Ruling verification uses an allowed-signers list supplied to the gate from a location the candidate cannot write. A copy in the repository is documentation, never the verification input.
- HC6. Uncertain is widening. The policy-diff classifier reports narrowing, widening, restructuring, or uncertain, and the gate treats uncertain exactly as widening.
- HC7. No baseline snapshots. There is no operation that records the current findings as acceptable. Inherited debt enters as individually scoped exception rulings with owners, reasons, and expiry.
- HC8. Instruments are pinned and recorded. Every external tool Warrant runs or ingests is named in `warrant/instruments.lock` with its version and, where it is a binary, its digest; the receipt records what actually ran; a mismatch is `analysis: incomplete`.
- HC9. No scores. No command emits a scalar quality score for a file, module, change, or repository. Counts of specific things are fine; a weighted composite is not.
- HC10. Structured output is versioned. Every JSON document Warrant emits carries a `schema_version`, has a published JSON Schema under `schemas/`, and changes only by adding fields within a major version.
- HC11. Agent output has no decoration. When stdout is not a terminal, or `--format json` is given, output is JSON or NDJSON with no ANSI sequences, no progress text, and no repeated repository context.
- HC12. One conformance fixture per hard control. Every rule with `enforcement: static` or `pattern`, every facet transition, every exit code, and every widening class has a fixture in `tests/conformance/` with a positive case, a negative case, and a case that breaks the control and must be caught.
- HC13. No network in core operations. `snapshot`, `inventory`, `model`, `query`, `check`, `gate`, `verify`, `policy`, and `rule` make no network calls. The judgment lane and `instrument install` are the only operations that may, and they say so.
- HC14. Atomic outputs. A cache entry, receipt, or model database is written to a temporary path and renamed into place; a partial write is never readable as complete.
- HC15. Filename globs scope contracts; they never express them. A contract's subject is a module, interface, effect, capability, or symbol; a glob may say where a module's files live and nothing else.
- HC16. Source tripwire. Warrant's own source is budgeted per crate in `scripts/budget.sh` and CI fails above the budget; a change that raises a budget includes an architecture note in the commit body. Budgets are alarms, not proofs.
- HC17. Every claim of completeness names its denominator. An output that says "no findings" also says how many files, modules, and contracts were evaluated and how many were not.

### 1.6 Definition of done for a Warrant capability

A capability is complete when: it is reachable from the CLI and, where applicable, the MCP surface; its structured output validates against its published schema; its conformance fixtures pass and its break-the-control fixture fails as designed; it has run on the frozen corpus without a regression in the corpus report; and its receipt or output names the tool build that produced it. A green unit-test run proves the unit; it does not prove the fixture, and neither proves the corpus run. Milestone exit criteria in section 21 are written in these terms.

## 2. Domain vocabulary

Seven nouns carry the product. Everything else in this document is defined in terms of them.

Snapshot. An exact, content-addressed set of files. A snapshot has a kind (`worktree`, `index`, `commit`, `lane`, `integrated`), a tree identity (the git tree object id, computed the same way for every kind so that any two snapshots are comparable), and a repository identity. Snapshots are immutable; a changed file is a different snapshot. Section 4.

Inventory. The classified list of every path in a snapshot and every path deliberately excluded from it, each with a class, the rule that assigned the class, and a reason. The inventory is the denominator of every claim Warrant makes. Section 5.

Program model. What the build believes about the code: modules, files, symbols, edges (imports, re-exports, dynamic loads, registrations, declared links), entrypoints, effects, and a capability report per language integration stating what it could and could not resolve. Stored as one SQLite database per snapshot. Section 6.

Contract. A declaration of architectural intent in the domain's vocabulary, with a kind (`module`, `dependency`, `interface`, `effect`, `state`, `capability`, `pattern`, `evidence`, `data`), an intent, an owner, an authority, an enforcement mode, and stated limits. The set of contracts in force at a snapshot, after compilation and conflict checking, is the effective policy, and its digest is the policy identity. Section 7.

Obligation and evidence. An obligation is one concrete checkable requirement that a contract produces when applied to one subject in one snapshot ("`billing` must not import `mail/adapter`"; "capability `undoable-operation` requires a readback test receipt for `debrief.undo`"). Evidence is whatever satisfies or fails it: a graph observation, a pattern match, an external receipt bound to the snapshot, or a valid exception ruling. Sections 8 and 9.

Finding and verdict. A finding is an unsatisfied obligation with an explanation that answers the seven questions in W19. A verdict is the result of evaluating a snapshot under an effective policy: four facets (compliance, analysis, evidence, approval), the findings, reviewable signals, and a derived acceptance state. Section 10.

Receipt. The verifiable record of a verdict: what was checked, against which exact inputs, by which tool build, with what result, at what clock. A receipt can be verified by a machine with no access to a model. Section 10.

Supporting terms, each defined where it is first used and collected here for lookup:

- Ruling: a signed record by which a human changes the policy, grants an exception, changes a trust root, or overrides a gate, with scope, policy digest, candidate range, expiry, and supersession. Section 11.
- Exception: a ruling of kind `exception` that satisfies a specific obligation for a specific finding identity, with a disposition (`false-positive`, `accepted-design`, `temporary-debt`). Section 11.4.
- Profile: a file that carries taste: measurable preferences as evidence obligations and judgment preferences as typed questions with thresholds. Section 16.
- Instrument: an external tool whose output Warrant runs or ingests, identified by name, version, and digest. Section 9.
- Lane, base, integrated candidate: a lane is one agent's line of work as a snapshot; the base is the approved snapshot it started from; the integrated candidate is the snapshot produced by merging lanes onto the base. Section 13.
- Effective policy: the compiled, conflict-checked set of contracts applicable at a snapshot. Section 7.6.
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
    snapshot/     git trees, worktree hashing, merge-tree integration, rename detection
    inventory/    classification rules, entrypoint discovery, generated and vendored provenance
    model/        SQLite schema, writers, read-only query surface
    lang-ts/      TypeScript and JavaScript integration (oxc, oxc_resolver, tsc parity, tsgo sidecar)
    lang-rust/    Rust integration (cargo metadata, syntax, use graph); second integration, proves the seam
    evidence/     attest wrapper, SARIF and JSON reporter ingest, evidence kinds, instrument pinning
    authority/    ruling records, canonicalization, sshsig signing and verification, trust roots
    judgment/     profile format, state builder, backend adapters (System One shape), calibration harness
    census/       observation, proposal, importers (dependency-cruiser, eslint boundaries)
    render/       map rendering (Mermaid, DOT, JSON), human output
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
- `warrant check` and `warrant gate` run to the end; `check` on a lane snapshot, `gate` on an integrated candidate with a trust root.
- `warrant context`, `warrant propose`, and `warrant explain` read the model and the effective policy and produce answers without producing a verdict.
- `warrant verify` reads a receipt and recomputes what it can without re-running instruments.

### 3.3 Seams and the stitch map

Each seam names the borrowed tool, the reason, and the alternatives that lost. The versions to pin are in section 19 and were verified by the research lanes on 2026-09-16.

| Seam | Borrowed | Why | Rejected |
|---|---|---|---|
| Snapshot identity and integration | git object model through the `gix` crate, falling back to the git binary for `merge-tree --write-tree` where `gix` lacks it | Git already content-addresses trees; a worktree hashed into a tree is comparable to any commit; `merge-tree --write-tree` produces an integrated candidate without touching the worktree | Own hashing scheme (would not be comparable to commits); libgit2 via `git2` (C dependency, no merge-tree); mtime-based change detection (lies) |
| TypeScript syntax and bindings | `oxc_parser`, `oxc_ast`, `oxc_semantic` | Rust-native, fast, gives per-file symbols, scopes, references, and export and import bindings; already proven in the predecessor | tree-sitter for TypeScript (no binding resolution); SWC (heavier, less semantic surface); the TypeScript compiler as the only parser (Node dependency on every check) |
| Module resolution | `oxc_resolver`, qualified against `tsc --traceResolution` on the frozen corpus | Handles tsconfig paths, package exports, conditions, symlinks; parity with the compiler is measured, not assumed | Own resolver (the predecessor's early mistake); tsc-only (slow, Node on the fast path) |
| Compiler-authority symbol references | TypeScript 7's native compiler through its LSP, or `@typescript/typescript6` plus an SCIP indexer, decided by spike S1 | The compiler is the only source that knows type-derived references; which surface to use is unproven as of 2026-09-16 | Reimplementing type inference (no) |
| Structural pattern contracts | ast-grep rules through the `ast-grep-core` and `ast-grep-config` crates over tree-sitter grammars | A pattern-enforced contract is an ast-grep rule with Warrant metadata; users already know the YAML | Semgrep (Python, licensing of the engine for embedding); CodeQL (license); hand-rolled matchers |
| Evidence from other tools | SARIF 2.1.0 through `serde-sarif`; JSON reporters for knip, Fallow, tsc, vitest, jest, Stryker; coverage as istanbul JSON and lcov | Normalize location and provenance, not meaning; every serious tool emits one of these | Writing detectors Warrant does not need to own |
| Receipts and rulings | in-toto Attestation Statement v1 with one Warrant predicate type per kind; RFC 8785 canonical JSON; detached SSH signatures (sshsig) under one namespace per kind; signing only ever by `ssh-keygen`, verification in-process with the `ssh-key` crate and `ssh-keygen -Y verify` as the oracle | Existing verification tooling, human-readable, works with the key Trey already has; the trust root's own `namespaces=` matching gives kind-scoped delegation for free | GPG (UX); Sigstore keyless (identity infrastructure; a later option for teams); signed commits as the approval channel (approves a tree, not a scoped statement) |
| Model store and query surface | SQLite through `rusqlite` (bundled), opened read-only for queries with an authorizer | One file per snapshot, every agent speaks SQL, no server | In-memory only (no query surface); a graph database (weight); JSON dumps (no queries) |
| Policy shape | YAML with a published JSON Schema, validated with `schemars`-generated schemas | Diffable, signable, readable by agents and humans; the schema is the lint | Rego (a second language); Cedar (permission model does not fit graph obligations; kept as a candidate for widening analysis); a TypeScript DSL (harder to sign and diff); CUE and Dhall (adoption) |
| Judgment lane | The System One request shape (state plus typed questions to calibrated typed answers) as the adapter interface; TypeSafe Jev as one backend; any LLM with structured output as another | Typed, calibrated, parallel, cheap enough to ask fifty narrow questions per hunk; vendor-neutral by shape | An LLM writing a paragraph review (unstructured, expensive, not diffable, not calibrated) |
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
9. The judgment-lane profile format, state builder, backend adapter, and calibration harness (section 16).
10. The census proposer and configuration importers (section 15).
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
  snapshots/<tree-id>/            inventory and model artifacts for that tree
  evaluations/<eval-key>/         verdicts keyed by (tree, policy digest, instrument set, profile digest)
  receipts/<receipt-id>.json      receipts produced here
${XDG_CONFIG_HOME:-~/.config}/warrant/
  allowed_signers                 the gate's trust root on this machine, if the gate runs here
  revoked_keys                    optional; OpenSSH revoked-keys format
  config.toml                     jobs, judgment backends (credentials by reference only), defaults
```

Nothing under the repository's `warrant/` is trusted for approval without signature verification against a trust root supplied from outside the candidate. The cache is content-addressed, so two worktrees of the same repository share entries only when their trees are identical, which is the isolation W23 asks for without any per-worktree bookkeeping.

### 3.6 Process and resources

Warrant runs as a single process that exits. Parallelism is a bounded `rayon` pool sized by `--jobs`, defaulting to half the available cores and never fewer than one, because a fleet of agents shares the machine (W25). SIGINT and SIGTERM cancel cooperatively: in-flight artifacts are discarded, nothing partial is renamed into place (HC14), and the exit code is 130 or 143. Memory is bounded by streaming file reads and by writing the model to SQLite as it is built rather than holding a whole-repository graph in memory. External instruments run as child processes with a wall-clock limit from `instruments.lock`, with stdout and stderr captured to files whose digests go into the receipt.

### 3.7 Trust boundaries

Three principals matter, and the design is honest about what each can do.

- The agent principal has full write access to the working tree, can run any command as its own user, can edit every file under `warrant/`, can regenerate any cache, and can fabricate any JSON. It cannot produce a valid signature for a key it does not hold.
- The signer principal holds a private key (in this estate, on a different Unix user with a passphrase) and signs canonical ruling records after reading a rendered effect summary. The signer never runs agent-controlled code as the signer principal; `warrant rule sign` is a small, dependency-light operation whose input is a file and whose output is a signature.
- The gate principal runs `warrant gate` on the integrated candidate with a trust root it obtained from outside the candidate: a file on the CI runner, a secret, or the signer's own machine. The gate verifies every ruling it relies on against that root, verifies every external receipt against the snapshot, and produces the receipt that the merge depends on.

What this does not protect against, stated plainly so nobody claims otherwise: a compromised gate runner; a signer who signs without reading; a truthful signature over an untruthful test (a receipt proves the test ran and what it reported, not that the test was good); and an agent that edits the Warrant binary on a machine where the gate trusts that binary. Section 11.7 lists the mitigations that exist and the ones that do not.

## 4. Snapshots

A snapshot answers "which exact bytes did Warrant look at," and every other artifact hangs off its identity. The design borrows git's object model wholesale: a snapshot is a git tree, whatever it was made from.

### 4.1 Kinds

- `commit`: the tree of a revision. `warrant snapshot --commit <rev>`.
- `index`: the staged tree, exactly what `git write-tree` would produce. `warrant snapshot --index`.
- `worktree`: the working tree, including untracked files that are not ignored, hashed into a temporary in-memory index and written as a tree. The repository's real index is never touched. `warrant snapshot --worktree` (the default when no kind is given).
- `lane`: any of the above, tagged with a lane name and the base tree it started from. `warrant snapshot --lane <name> --base <rev> [--commit <rev>|--worktree]`.
- `integrated`: the tree produced by merging one or more lanes onto a base without touching the working tree, through `git merge-tree --write-tree`. Lanes are folded onto the base in the order given, and the order is recorded. `warrant snapshot --integrated --base <rev> --lane <rev> [--lane <rev>...]`.

A merge conflict does not produce a snapshot. The result is a structured error naming the conflicting paths, and a `warrant gate` asked to evaluate an unbuildable integrated candidate reports `analysis: incomplete` with reason `integration-conflict`, never `compliance: fail`. Conflicts are a coordination problem for the orchestrator, and Warrant reports them rather than adjudicating them (W22).

### 4.2 Identity

A snapshot identity is `{object_format}:{tree_id}`, where the object format is the repository's (`sha1` or `sha256`) and the tree id is the git tree object id. Two snapshots with the same identity contain the same bytes at the same paths with the same modes; this is git's guarantee and Warrant adds nothing to it. The repository identity is the object id of the root commit (the lexicographically first if there are several), so that caches and receipts from different clones of the same repository are comparable.

Repositories without git are not supported in v1. The vision commits to git as the snapshot store, and a directory without history has no base to integrate against and no identity a receipt can name. `warrant snapshot` on a non-repository exits 2 with an explanation.

### 4.3 What a snapshot excludes, and how the exclusion is recorded

- Ignored files (per `.gitignore`, `.git/info/exclude`, and the global excludes file) are not in the tree. The inventory records how many were ignored under each top-level directory so that "the analysis is clean" is never confused with "the analysis did not see the build output."
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
  "kind": "integrated",
  "tree": "sha1:71ab...",
  "object_format": "sha1",
  "base": "sha1:5e10...",
  "lanes": [
    { "name": "undo-grouping", "tree": "sha1:c0de...", "base": "sha1:5e10..." },
    { "name": "calendar-sync", "tree": "sha1:beef...", "base": "sha1:5e10..." }
  ],
  "merge_order": ["undo-grouping", "calendar-sync"],
  "excluded": { "ignored_files": 4211, "submodules": 1, "oversize": 0 },
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

Rules come from four places and are compiled together: defaults shipped with each enabled integration (for TypeScript, for example, `*.test.ts` and `__tests__/**` are `test`, `*.d.ts` is `schema`); declarations in `warrant.yaml` (`inventory.classes`); the `files` selectors of `module` contracts, which assign both class `source` and a module; and explicit per-path overrides. Two rules that assign different classes to the same path are a policy lint error, not a precedence question. There is no first-match-wins and no last-match-wins; ambiguity is a configuration defect the author fixes.

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
- `analyze(unit) -> files, symbols, references, edges, unsupported`: per-file symbol tables, exported bindings, import and re-export bindings, in-file references to imported bindings, dynamic-load sites, and every construct it could not analyze with a reason.
- `resolve(edge) -> target file | external package | unresolved(reason)`: module resolution under the unit's configuration.
- `entrypoints(unit) -> entrypoints`: what the integration can observe (package manifests, test files, framework adapters it ships).
- `capabilities() -> CapabilityReport`.

The capability report is embedded in the model, printed by `warrant model --capabilities`, and copied into every receipt:

```json
{
  "integration": "lang-ts",
  "version": "0.1.0",
  "instruments": { "oxc_parser": "…", "oxc_resolver": "…", "typescript": "7.0.2 (parity-qualified)" },
  "resolution_authority": "parity-qualified",
  "symbol_level": "binding",
  "type_only_distinction": true,
  "supports": ["esm-import", "esm-reexport", "cjs-require-literal", "dynamic-import-literal", "tsconfig-paths", "package-exports", "project-references"],
  "unsupported": [
    { "construct": "dynamic-import-nonliteral", "treatment": "unresolved-dynamic" },
    { "construct": "reflection-registration", "treatment": "requires-declaration" },
    { "construct": "type-derived-reference", "treatment": "not-observed" }
  ],
  "limits": "Consumers of a symbol are files that import its binding directly or through re-export chains. References that arise from type inference, dependency injection, or reflection are not observed."
}
```

`resolution_authority` is one of `native` (the language's own compiler produced the resolution), `parity-qualified` (a faster resolver whose agreement with the compiler was measured on the frozen corpus for the configuration features this repository uses), `unqualified` (the repository uses configuration features outside the qualified set), or `syntax-only`. `symbol_level` is `none`, `binding`, or `compiler`. An `unqualified` authority sets `analysis: incomplete` with reason `resolution-unqualified` unless a ruling accepts it for a named configuration feature; this is W02's compatibility profile made explicit.

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

The model digest is the sha256 of a canonical dump of these tables in a fixed order, so that two builds of the same snapshot with the same integrations produce the same digest and a differing digest means the instrument changed (W31).

### 6.3 Modules

Modules are the unit of architectural intent. They are declared in `module` contracts (section 7.4) with a `files` selector (HC15), or proposed by the census. Every source file belongs to at most one module; overlapping selectors are a policy lint error; an unassigned source file is unowned (section 5.2). A module's public interface is the set of its exported symbols that its `interface` declaration names, or, when no declaration exists, the set of symbols imported from it by files outside the module (an observed interface, labeled as such).

### 6.4 Edges and the dynamic forms

Edges are recorded with their basis. Static ESM imports and re-exports, CommonJS `require` with a literal specifier, and `import()` with a literal specifier resolve normally. Type-only imports (`import type`, `import { type X }`) are recorded with `type_only = 1` and are excluded from dependency contracts unless the contract says `include_type_only: true`.

A dynamic load whose target cannot be determined statically is recorded as `unresolved` with reason `dynamic-nonliteral` and listed under the file's unsupported constructs. The obligation this creates is on the file's module: either a `declared` edge exists (a `dependency` contract's `declares:` entry naming the target, reviewed and signed like any contract) or the module's contract sets `dynamic_loads: unresolved-allowed`, which is a widening-classified setting. With neither, the analysis facet is incomplete (`unresolved-dynamic`). Registrations, dependency injection, and reflection follow the same rule through `registry` declarations (section 7.5): what a declaration says is treated as declared evidence and labeled as such; nothing is inferred from a function's name.

### 6.5 The binding-level symbol graph

In v1 the TypeScript integration builds a binding-level graph from oxc: every exported binding of every file, every import binding and the export it resolves to (following re-export chains, including `export * from`), every in-file reference to an imported binding, and every dynamic-load site. This answers "which files depend on symbol X" for every direct and re-exported import, which is the question most contracts ask (interface consumers, effect callers through an adapter function, dependency direction). It does not answer questions that need types: a method called on an instance whose class was imported elsewhere, a callback registered through a framework, a reference reachable only through inference. The capability report says so, and a contract that needs compiler-level references says `limits: binding-level` in its own text until spike S1 lands.

### 6.6 Spike S1: compiler-authority references

Question: which surface of the TypeScript 7 toolchain should supply compiler-level references and definitions in batch, without a permanent dependency on an unstable API? Candidates: the native compiler's language server queried over LSP for `documentSymbol`, `references`, and `definition`; the `@typescript/typescript6` compatibility package driving a SCIP indexer (`scip-typescript`) to produce an index Warrant reads; and the tsgolint bridge as a pattern for invoking the Go compiler. Exit criteria: on the frozen corpus, for one hundred sampled exported symbols with hand-verified consumer lists, precision and recall both at or above 0.98; a full index of a repository the size of Atlas completing within the gate's instrument time limit; no network; a pinned, recorded instrument identity. Fallback if none passes: binding-level stays, and contracts that need more are labeled. The spike's report goes in `docs/research/` and its decision in section 20.

### 6.7 The Rust integration

The second integration exists to prove the seam is not TypeScript-shaped and to let Warrant check itself. Units come from `cargo metadata`; the crate dependency graph is authoritative from the same source; modules are crates and `mod` trees; `use` edges are extracted syntactically and resolved within the crate at binding level; visibility (`pub`, `pub(crate)`) is the interface signal; entrypoints are binary targets, tests, benches, `build.rs`, and proc macros. Its capability report says `resolution_authority: native` for crate edges (cargo is the compiler's own source of truth) and `symbol_level: binding` for `use` edges. Compiler-level references through rust-analyzer are a later spike. The research report `docs/research/2026-09-16-polyglot-integrations.md` carries the tool choices.

### 6.8 Parity qualification

`warrant instrument qualify lang-ts` runs the repository's pinned TypeScript compiler with `--traceResolution` (through a Node sidecar that runs and exits) over the frozen corpus and the current repository, compares every resolution to `oxc_resolver`'s, and writes a parity report listing the tsconfig features exercised and the agreement rate. The integration's capability report is `parity-qualified` only for the feature set the report covers; a repository that uses a feature outside that set is `unqualified` until qualification is rerun with the corpus extended. The predecessor's `doctor compare` did this ad hoc; here it is the mechanism that decides the authority label, and its report is an instrument artifact with a digest in the receipt.

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

- An in-memory graph only (the predecessor's `petgraph` model). It could not be queried by agents, could not be reused across commands, and had no identity. `petgraph` is still used inside evaluation for SCC and path algorithms over data loaded from SQLite.
- Compiler-only analysis (tsc for everything). Correct and slow, with a Node process on every lane check; it remains the authority path for references (spike S1) and the parity oracle for resolution.
- tree-sitter for TypeScript. No binding resolution; it is the right lowest layer for languages without an integration and for ast-grep patterns, and the wrong one for a program model.
- Inferring registrations from function names (`register*`, `use*`). Exactly the guess the vision forbids; declarations replace inference.
- A graph database. One more server, one more query language, no agents that speak it natively.

## 7. Contracts and the policy language

Contracts are how architectural intent becomes executable. The language is small, declarative, and typed; it is not a programming language, and W05's warning that policy must not become "a second application written in a poorly supported programming language" was the constraint I kept in front of me while designing it.

### 7.1 Principles

- A contract speaks the domain's vocabulary: modules, interfaces, effects, state, capabilities. Filename globs appear only to say where a module's files live (HC15).
- Every contract carries its intent (why), its owner (who answers for it), its authority (which ruling established it), its enforcement mode (how Warrant checks it), and its limits (what that check cannot see). A contract without limits is rejected by lint, because every enforcement has some.
- Every contract has a class: `invariant` (a property of the product), `preference` (a property of the code's style, still deterministic), or `migration` (temporary, with an expiry). The class decides the default consequence of a violation.
- Conflicts are configuration defects, never resolved by order. Precedence exists only as an explicit `overrides` that names the contract it overrides and carries its own authority.
- The compiled result, the effective policy, is a canonical document with a digest. Two authors who write different YAML that means the same thing get the same digest; an author who changes meaning gets a different digest and a classified diff (section 11.5).

### 7.2 Files

`warrant/warrant.yaml` is the manifest:

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

Common fields on every contract: `id`, `kind`, `intent`, `owner`, `authority` (`ruling:<id>` or `draft`), `class` (default `invariant`), `expires` (required when `class: migration`), `enforcement` (`static`, `pattern`, `evidence`, `mixed`), `limits`, `on_violation` (`fail`, `review`, `note`; defaults `fail` for invariants, `review` for preferences, `fail` for migrations), `supersedes` (a contract id), `overrides` (a contract id, requires `authority: ruling:`).

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
  intent: Email leaves the building only through the Gmail adapter, only from the send workflow, only with a request context.
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
  limits: Permitted callers are proven at binding level. The context check recognizes a first argument named ctx; it does not prove a transaction is open. Effect control is proven by the named test receipt on this snapshot, not by static analysis.
```

`state`. Declares who may write a named store and how write sites are recognized.

```yaml
- id: state.approvals
  kind: state
  intent: Approval rows are created and consumed only by the dispatcher.
  store: approvals
  writers: { modules: ["core.actions"] }
  write_sites:
    pattern: { rule: { pattern: "db.insert(approvals)" } }
  enforcement: pattern
  limits: Write sites are recognized by the ORM call shape. Raw SQL strings and dynamic table references are not recognized and are listed as a known gap.
```

`capability`. Links the parts of a user-visible feature; each registered instance must satisfy every link.

```yaml
- id: capability.undoable-operation
  kind: capability
  intent: An undoable operation has one entrypoint, an authorization check, one compensation owner, a readback test, and a failure-path test.
  owner: trey
  authority: ruling:2026-09-20-bootstrap
  instances: { registry: undo-registry }
  requires:
    - { link: entrypoint, basis: registry }
    - { link: authorization, pattern: { rule: { pattern: "plan($$$)" }, within: instance } }
    - { link: compensation-owner, one_of_module: core.actions.undo }
    - { link: readback-test, evidence: { kind: test-receipt, tag: "undo:{instance}" } }
    - { link: failure-path, evidence: { kind: test-receipt, tag: "undo-failure:{instance}" } }
  enforcement: mixed
  limits: The authorization link recognizes a call to plan inside the instance's function body; it does not prove the call guards the effect. Tests prove what the named receipts say they prove.
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

```yaml
- id: data.private-mail
  kind: data
  intent: Private mailbox content reaches only the model boundary and the memory summarizer, and summaries are not sensitive.
  sources: { symbols: ["core.gmail::fetchThread", "core.gmail::fetchMessage"] }
  permitted_sinks: { modules: ["core.model", "core.memory"] }
  transformations:
    - { symbols: ["core.memory::summarize"], removes_sensitivity: true }
  enforcement: pattern
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

`warrant policy compile` (run implicitly by every evaluating command) loads every policy file named by the manifest, validates each against the published schema (unknown fields are errors with a nearest-name suggestion; unsupported schema versions are errors), assigns and checks ids for uniqueness, resolves every selector against the inventory and the model, checks conflicts, normalizes the result into a canonical JSON document, and computes its sha256 digest. The digest is the policy identity in every verdict and receipt.

Conflict classes, each a lint error:

- Two `module` contracts whose `files` selectors overlap.
- Two `dependency` contracts where one allows and the other denies the same edge, without an `overrides` between them.
- Two `state` contracts naming the same store with different writers.
- An `interface` that lists exports the module does not export.
- A `capability` whose registry has no instances (unless `may_be_empty`).
- A contract whose `enforcement: static` asks for something the enabled integrations report as unsupported (for example, a consumer check that needs compiler-level references while the capability report says `binding`), reported as `enforcement-unsupported` with the integration's stated treatment. The author either lowers the claim (`limits` and `enforcement: pattern`) or the integration is upgraded; the contract does not silently run at a weaker level.
- A `migration` without `expires`, or an expired migration still present (reported; the contract stops applying at expiry and the gate reports it as `expired-contract`).
- An `overrides` without `authority: ruling:`.

Drift checks (W09), reported by `warrant policy lint` as findings of class `drift`, never silently: modules with no files; declared interfaces with no consumers anywhere, including declared external consumers (reported, not blocking, because a new interface has none yet); registries with no matches; stores with no recognized write sites; capabilities whose instances all fail the same link (which usually means the pattern is wrong, not the code); declared external consumers whose symbols no longer exist.

Unratified contracts. A contract with `authority: draft` is one an agent or a person wrote without a ruling. It is compiled and enforced (so that a narrowing an agent proposes takes effect in its own lane immediately), and its effect on the policy relative to the base is classified. A draft that widens sets `approval: required` at the gate; a draft that narrows is listed under the verdict's `unratified` array with the classification `narrowing`, so that the signer sees new rules arriving and can ratify them in one ruling. This is how W15's "permission to edit is not permission to approve" is implemented without blocking an agent from tightening its own rules.

### 7.7 Explaining the effective policy

`warrant policy effective <path|symbol|module>` prints every contract that applies to the subject, the selector that matched it, its authority and class, and, for dependency questions, the specific allow or deny entry that governs, with the ruling that established it. `warrant policy diff` is section 11.5.

### 7.8 A short worked policy

For a repository shaped like Atlas, the whole architecture in section 1.5 of that product's spec compiles to roughly: one `module` contract per bounded context and per deployable; one `dependency` contract for `core`'s independence and one for the transports; one `interface` contract for the dispatcher; three `effect` contracts (send, delete, external model call); one `state` contract per ledger table; two `capability` contracts (tool, undoable operation); two `evidence` contracts (the gates); a handful of `pattern` preferences; and the declarations for the three registries and the ledger stores. The census (section 15) proposes the modules and dependency contracts from observation; the effects, capabilities, and evidence requirements are written by hand, because they encode intent the code cannot show.

### 7.9 Considered and rejected

- Rego and OPA. A general policy language with its own evaluation semantics; agents would have to learn it, and its verdicts are opaque to a human reading YAML. Warrant's policy is data, and its semantics are the compiler's, tested by fixtures.
- Cedar. An excellent permission language with formal analysis, but its model (principal, action, resource) does not express graph obligations like "every registered instance links to a compensation owner." It remains the leading candidate for a future permission-widening analysis over compiled `dependency` contracts.
- A TypeScript or Rust DSL for policy. Expressive, and exactly the wrong thing: policy that is code is harder to sign, to diff semantically, and to keep out of the agent's pen.
- Built-in layer taxonomies ("domain, application, infrastructure"). W's "not a rigid taxonomy" objection stands; layers are one shape of `dependency` contract, not a primitive.
- Severity levels. The predecessor had `error`, `warning`, `info` per rule; `warning` is how incomplete analysis became success. Warrant has `on_violation` with three consequences, none of which is "print and pass."

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

Signals are reviewable indicators that are neither findings nor facets. They are computed by comparing the candidate to its base, they carry numbers and locations, and they never carry a score (HC9). The profile decides whether a signal is `note` or `review` (which sets `approval: required`).

Metric-gaming signals (W13): exclusions widened (from the policy diff); inventory shrank in a class (files reclassified out of `source` or `test`); tests removed from a unit named by an `evidence` contract; assertion count decreased in changed test files (counted by pattern per language: `expect(`, `assert`, `#[should_panic]`); suppressions added (`eslint-disable`, `@ts-ignore`, `@ts-expect-error`, `as any`, `#[allow(`, per-language patterns); type escapes added; helper extraction that moves a complexity finding rather than removing it (when Fallow or a complexity instrument reports the same total across the moved functions).

Conceptual-expansion signals (W11): new public symbols per module; new modules; new declared stores (state owners); new module-to-module edges; new cycles; new configuration knobs (declared patterns for environment reads and config keys); changes that touch more modules than the capability they claim to complete (change locality, computed from the capability instances the diff touches); interfaces whose consumers now need to know more (new required parameters on exported functions, counted syntactically).

Every signal is presented with the counterexample that could justify it, in the profile's words, so that a reviewer can reject a cleanup that improves numbers and hurts the design (story E) and accept one that deletes tests because the tested thing no longer exists.

### 8.6 Stable identity

`finding.id` is the first sixteen characters of a base32 sha256 over the contract id, the obligation type, the subject identity, and the basis kind. Subject identity is structural: a module id, a module-to-module edge, a module-qualified symbol, a capability instance name, or a store name. Line numbers never enter the identity. Between a base and a candidate, git rename detection maps old paths to new ones for file-based subjects, and a symbol keeps its identity when its export name and module are unchanged; a rename that git cannot attribute with confidence produces a new id with `renamed_from: uncertain`, which is honest and which W16 asks for ("a renamed file should carry an exception only if its identity and meaning are established").

### 8.7 Considered and rejected

- Severity levels on findings. Replaced by `consequence`, which is the contract author's decision about what happens, not a label about how bad it looks.
- Autofix patches. W20 allows small inspectable patches bound to a snapshot; v1 emits suggestions and the focused check, and a patch-producing mode is deferred until findings are stable enough to bind a patch to.
- Positional fingerprints as the primary identity. The predecessor kept both a stable and a positional fingerprint; the positional one is what made a moved line look like a new finding. Position is location, not identity.

## 9. Evidence and instruments

Warrant relates evidence to obligations; it does not manufacture evidence. This section defines what counts, how it is bound to a snapshot, and how the tools that produce it are pinned.

### 9.1 Evidence kinds

- `graph`: a static observation from the program model (an edge, a symbol, an entrypoint). Basis `observed-static` or `declared`.
- `pattern`: an ast-grep match or non-match within a scope.
- `test-receipt`: a record produced by `warrant attest run` that a command ran on an exact snapshot and what it reported.
- `report`: an instrument's output (SARIF or a known JSON reporter) imported and bound to a snapshot.
- `observation`: a receipt that something was observed on a named deployed revision, produced by a runner that signs it.
- `exception`: a valid exception ruling (section 11.4).
- `judgment`: a typed model answer from the judgment lane. Judgments can satisfy no compliance obligation and can only set `approval: required` or attach advice (HC1).

Test evidence has a class, and an obligation names the class it requires: `local` (ran on the developer's machine against fakes), `mocked` (ran with mocked external services), `integration` (ran against real local services, such as a real database), `deployed` (observed on a named deployed revision). A lower class never satisfies a higher requirement; a receipt states its class and the obligation states the minimum (W08).

### 9.2 The attest wrapper

`warrant attest run --tag <tag> --class <class> [--unit <unit>] [--snapshot worktree|index|<rev>] -- <command...>` takes the snapshot, runs the command, and writes a receipt:

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
  "exit_code": 0,
  "stdout_digest": "sha256:…",
  "stderr_digest": "sha256:…",
  "duration_ms": 41230,
  "instruments": [ { "name": "vitest", "version": "…", "source": "package-lock" } ],
  "started_at": "2026-09-16T21:10:02Z",
  "finished_at": "2026-09-16T21:10:43Z",
  "signature": null
}
```

The receipt records the tree id at the start of the run and verifies it again at the end; a tree that changed during the run produces a receipt with `snapshot_unstable: true`, which satisfies nothing. Environment values are recorded only for an allowlist of names in the manifest; other names are recorded as present or absent, never with values (W25's "without leaking source or secrets"). Stdout and stderr contents are stored in the cache by digest for `warrant explain` to show, and only their digests enter the receipt.

An `evidence` obligation is satisfied by a receipt whose tag, class (at or above the minimum), and unit match and whose tree id equals the snapshot being evaluated. A receipt for any other tree is `stale` and is listed as such in the evidence facet, with the tree it was for, so that an agent can see exactly which proof another agent's change invalidated (story D, W22). Section 13.3 defines the one case in which a lane's receipt carries over to an integrated candidate.

### 9.3 Instrument reports

`warrant evidence import --instrument <name> [--snapshot <kind>] <file>` binds an instrument's output to a snapshot. SARIF 2.1.0 is the preferred format; the known JSON reporters of knip, Fallow, tsc, ESLint, oxlint, vitest, jest, and Stryker are mapped by small adapters that are themselves versioned instruments. Import records the file's digest, the instrument's identity from the lock, and the tree id, and stores each result as an evidence row keyed by instrument, rule id, and location. A report that is truncated, fails to parse, or does not name the tree it was produced for is `report-truncated` or `report-unbound` and sets the analysis facet incomplete when a contract requires that instrument.

Contracts reference instrument evidence through `evidence` requirements that describe the absence or presence of results in a scope, for example `{ kind: report, instrument: knip, rule: "unused-export", absent: true, scope: { modules: ["core.*"] } }` or `{ kind: report, instrument: stryker, metric: "mutationScore", at_least: 0.8, unit: "apps/worker" }`. Meaning is not normalized across instruments (W12): a knip result and a Fallow result about the same symbol are two evidence rows that corroborate, not one deduplicated row, and the explanation shows both.

### 9.4 The instruments lock

`warrant/instruments.lock` pins every external tool Warrant runs or ingests:

```yaml
schema_version: warrant.instruments/1
instruments:
  - { name: typescript, kind: npm, version: "7.0.2", role: parity-oracle, required: true, timeout_s: 600 }
  - { name: knip, kind: npm, version: "…", role: report, required: false, invocation: ["npx", "knip", "--reporter", "json"] }
  - { name: fallow, kind: binary, version: "…", digest: "sha256:…", role: report, required: false }
  - { name: ast-grep-rules, kind: embedded, version: "warrant-0.1.0" }
  - { name: lang-ts, kind: integration, version: "0.1.0", qualified_on: "corpus:2026-09-16" }
  - { name: nextjs-routes, kind: adapter, version: "0.1.0" }
```

`warrant instrument status` compares the lock to what is installed and what the receipts say ran. A required instrument that is missing, or present at a different identity, sets `analysis: incomplete` with `instrument-missing` or `instrument-mismatch` (HC8). There is no flag that downgrades this to a warning; the fix is to install the pinned version or to change the lock through a ruling.

### 9.5 Instrument upgrades

`warrant instrument upgrade <name> <version>` runs the frozen corpus (section 17.3) under the old and the new identity and writes a before-and-after report to `docs/instruments/<date>-<name>-<old>-<new>.md`: findings added, removed, and changed by contract; model digest changes per corpus repository; parity changes for resolvers; wall-clock and memory. The lock is not changed by this command. Adopting the new version is a change to the lock, which the policy-diff classifier treats as `uncertain`, which is widening (HC6), which requires a ruling whose rendered effect summary is that report. A tool getting more permissive shows up as an instrument change, never as an improvement in the code (W31, story F). The same procedure applies to Warrant's own version: `warrant self-qualify` compares corpus output between the installed build and a candidate build.

### 9.6 Considered and rejected

- Running tests inside Warrant. It is not a test runner (W12); the attest wrapper records, it does not execute test logic.
- Accepting "tests passed" as a boolean from CI. Unbound to a tree and to a class, it is exactly the laundering W24 describes.
- Normalizing instrument findings into one taxonomy. Location and provenance normalize; meaning does not, and pretending otherwise hides disagreement between instruments.
- Auto-updating instruments. A quiet upgrade is a quiet policy change.

## 10. The verdict and the receipt

### 10.1 Four facets

A verdict carries four facts, each computed independently, none able to override another:

- `compliance`: `pass` or `fail`. `fail` when at least one finding has `consequence: fail` and no valid exception.
- `analysis`: `complete` or `incomplete`, with the list of reasons from section 5.5 and section 8.2.
- `evidence`: `satisfied`, `missing`, or `stale`, with the list of required receipts and their status.
- `approval`: `none`, `required`, or `granted`, with the list of items that require a ruling: widening or uncertain policy changes, draft contracts that widen, findings with `consequence: review`, signals routed to review by the profile, judgments over a review threshold, instrument changes, and expired rulings still relied on.

`acceptance` is derived and never stored as an input: `accepted` when compliance is `pass`, analysis is `complete`, evidence is `satisfied`, and approval is `none` or `granted`; otherwise `blocked`, with the responsible facets named. A run can be `compliance: fail` and `analysis: incomplete` at the same time, and the verdict shows both (W17).

Lane checks (`warrant check`) produce verdicts of `mode: lane` whose acceptance field is `lane-clean` or `lane-blocked` and whose receipts carry a different predicate type from gate receipts. Nothing a lane check produces can be mistaken for gate acceptance, by format, not by convention (W17's "exploratory mode must not produce an artifact that can be confused with full acceptance").

### 10.2 Exit codes

- 0: `accepted` (gate) or `lane-clean` (check).
- 1: `blocked` or `lane-blocked`, for any facet. The JSON says which.
- 2: the run could not evaluate: invalid invocation, policy compile error, snapshot unbuildable, no trust root given to a gate.
- 3: internal failure or an instrument crash that is not a policy matter.
- 130 and 143: cancelled by signal; no artifact written.

One code for "not accepted" is deliberate. Agents read the JSON; shell scripts and CI need one branch; and distinguishing "blocked by findings" from "blocked by missing evidence" in the exit code would invite a script to treat one of them as softer. The facets are in the document; the exit code says whether the merge may proceed.

### 10.3 The verdict document

```json
{
  "schema_version": "warrant.verdict/1",
  "mode": "gate",
  "snapshot": { "…": "section 4.5 manifest" },
  "policy": { "digest": "sha256:…", "files": [ { "path": "warrant/policy/core.yaml", "blob": "sha1:…" } ], "unratified": [ { "contract": "style.no-todo", "class": "narrowing" } ] },
  "profile": { "path": "warrant/profiles/trey.yaml", "digest": "sha256:…" },
  "inventory": { "digest": "sha256:…", "summary": { "…": "section 5.5" } },
  "model": { "digest": "sha256:…", "capabilities": [ { "…": "section 6.1" } ] },
  "instruments": [ { "name": "typescript", "version": "7.0.2", "matched_lock": true } ],
  "compliance": { "status": "fail", "failing": ["f_7q3k9m2xw4pbz1a"] },
  "analysis": { "status": "complete", "reasons": [] },
  "evidence": { "status": "missing", "required": [ { "contract": "evidence.worker-gate", "status": "satisfied", "receipt": "tr_9v2…" }, { "contract": "capability.undoable-operation#undo:group_debrief", "status": "missing" } ] },
  "approval": { "status": "required", "items": [ { "kind": "policy-widening", "change": "dependency layering.core-independent: deny list shrank by @dbos-inc/*", "class": "widening", "ruling": null } ] },
  "acceptance": "blocked",
  "blocked_by": ["compliance", "evidence", "approval"],
  "findings": [ { "…": "section 8.3" } ],
  "signals": [ { "kind": "suppressions-added", "count": 3, "locations": ["…"], "consequence": "review" } ],
  "judgments": [ { "question": "single-responsibility", "subject": "apps/worker/src/workflows/undo.ts#groupUndo", "probability": 0.81, "threshold": 0.75, "consequence": "review", "backend": "typesafe:jev-1.12" } ],
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
    "policy": { "digest": "…", "files": [ "…" ] },
    "inventory": { "digest": "…", "summary": { "…": "…" } },
    "model": { "digest": "…", "capabilities": [ "…" ] },
    "instruments": [ "…as ran, with digests…" ],
    "evidence": { "receipts": [ { "id": "tr_…", "digest": "sha256:…", "tree": "sha1:…" } ], "reports": [ { "instrument": "knip", "digest": "sha256:…" } ], "exceptions": [ { "ruling": "r_2026-09-18-clone-3", "signature_verified": true, "principal": "trey", "valid_at_clock": true } ] },
    "judgments": { "profile_digest": "…", "backend": "typesafe", "status": "obtained", "entries": [ { "question_digest": "…", "state_digest": "…", "response_model": "jev-1.12", "answer": { "type": "noul", "noul": "0.81" }, "request_id": "…", "usage": { "input_tokens": 2140, "output_tokens": 9 }, "state_truncated": false } ] },
    "verdict": { "compliance": "fail", "analysis": "complete", "evidence": "missing", "approval": "required", "acceptance": "blocked", "finding_ids": [ "…" ], "signal_digest": "…" },
    "trust_root": { "digest": "sha256:…", "principals": ["trey"] },
    "evaluated_at": "…", "clock_source": "system",
    "tool": { "name": "warrant", "version": "…", "binary_digest": "sha256:…" },
    "uncaptured_inputs": ["locale", "filesystem case sensitivity (linux: sensitive)"]
  }
}
```

The receipt is written as RFC 8785 canonical JSON, UTF-8, no trailing newline, and the receipt id is the sha256 of those bytes. Verification compares bytes and never re-serializes; a receipt that has been reformatted fails at step 1 below. Signed records carry no JSON floats: counts are integers and every other number, including a judgment probability, is a decimal string exactly as the backend returned it, because RFC 8785 number serialization is the part of the standard implementations most often get wrong (`docs/research/2026-09-16-signing-and-attestation.md`, section 2).

The subject digest set carries the git tree id (the `gitTree` algorithm name from the in-toto DigestSet registry) and Warrant's own sha256 inventory digest, and the annotations name the identity kind (`worktree`, `index`, `commit`, `lane`, `integrated`) and the repository object format, because a SHA-1 and a SHA-256 repository produce different tree ids for identical content. Two receipts describe the same snapshot only when both digests match.

A gate runner signs the receipt with its own SSH key (sshsig, namespace `warrant-receipt@<domain>`, detached `.sig` beside the file), which makes "this receipt was produced by the gate" verifiable; `mode` is `gate` only for such a receipt. A receipt from a lane run is `mode: lane`, is unsigned or signed by the agent's own key (which the gate never trusts, but which lets a human see which agent produced it), and satisfies no gate obligation. A replay (`--now` supplied) is `mode: replay` and is never a gate receipt. The predicate type URI and the namespace domain are one decision (D2); both embed in signed bytes, so they are chosen once.

### 10.5 Verifying a receipt

`warrant verify <receipt.json> [--trust-root <allowed_signers>] [--recompute]` checks, in order, and reports each step's result:

1. Bytes and schema: the file parses as a Statement with a known `predicateType`, is byte-identical to its own RFC 8785 canonical form, and its digest equals its id.
2. Subject: the tree exists in the local repository (skipped with `--offline`, reported as unchecked).
3. Policy binding: the policy files at that tree hash to the recorded blobs and compile to the recorded policy digest.
4. Inventory binding: with `--recompute`, the inventory at that tree recomputes to the recorded digest; without, the recorded summary is reported as unverified.
5. Model binding: with `--recompute` and the pinned integrations installed, the model digest recomputes; otherwise unverified.
6. Instruments: recorded identities match the lock at that tree.
7. Evidence: every referenced test receipt and report file that is present has the recorded digest, and its own declared subject equals the receipt's snapshot digest; evidence with no subject digest is `unbound` and fails (W24), and evidence whose subject names a different tree fails even when its file digest matches, because that is exactly the case where a test ran against something else. Every referenced ruling verifies against the supplied trust root under the namespace for its kind, with the key's validity window checked at the ruling's `signed_at` and its expiry checked at the recorded evaluation clock; a ruling that verified then but has since expired or been revoked makes the receipt `stale`, which is distinct from `failed`.
8. Consistency: the facets recompute from the recorded findings, evidence statuses, and approval items.
9. Producer signature: if present, verifies against the trust root under `warrant-receipt@<domain>` with the principal the receipt names; an invalid signature is `failed`, an absent one makes the result `advisory`.

The result is `verified`, `verified-with-unchecked` (naming what was skipped), `stale` (naming the ruling), `advisory` (a lane or replay receipt, or an unsigned gate receipt), or `failed` (naming the step), and the exit code distinguishes them. No model is consulted at any step (W24). What verification cannot establish is stated in the output every time: that instruments ran honestly (only a signed runner attests that), and that the tests were adequate (nothing attests that).

### 10.6 Human rendering

When stdout is a terminal, or with `--format human`, the verdict renders as the paragraph the vision promises: what changed since the base (modules touched, capabilities affected), which obligations were satisfied and by what, what is uncertain (the analysis reasons and stale evidence), and whether any permission widened, followed by the primary findings with their focused checks. It is a rendering of the JSON; it contains nothing the JSON does not.

### 10.7 Considered and rejected

- A single pass or fail. The predecessor's `VerdictStatus::{Pass, Fail}` is exactly the collapse the vision forbids.
- A `warning` status. It is how "the parser failed on three files" became a green run.
- Distinct exit codes per facet. Rejected in section 10.2.
- A bespoke receipt format. in-toto's Statement costs nothing, has verification tooling, and lets a receipt sit beside SLSA provenance in the same store.
- Asking a model whether a receipt looks right. Never.

## 11. Rulings, signing, exceptions, and policy change

### 11.1 The ruling record

A ruling is the only way policy changes, an exception is granted, a trust root changes, an instrument is adopted, or a gate is overridden. It is an in-toto Statement, like a receipt, so that one canonicalization, one signature carrier, and one verification path serve both. Its subject is the thing the ruling is about (the policy digest it binds to, the tree it overrides, the trust root it replaces), and its `predicateType` names the kind, so the kind is inside the signed bytes and cannot be relabeled after signing.

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [ { "name": "policy", "digest": { "sha256": "…" } } ],
  "predicateType": "https://warrant.dev/attestation/ruling/exception/v1",
  "predicate": {
    "id": "r_2026-09-18-clone-exception-3",
    "statement": "The two implementations of retry backoff in worker/ingest and worker/send are intentionally separate until the ingest rewrite lands; consolidating now would couple their release cadence.",
    "scope": { "modules": ["worker.ingest", "worker.send"] },
    "findings": ["f_3n9q…", "f_8k2w…"],
    "applies_to_count": 2,
    "disposition": "temporary-debt",
    "candidate_range": { "base": "sha1:5e10…", "branches": ["main"] },
    "expires": "2026-12-01T00:00:00Z",
    "supersedes": null,
    "rendered_digest": "sha256:…",
    "signer": "trey",
    "drafted_by": "claude-code:fable:session-7a19",
    "created_at": "2026-09-18T15:02:00Z",
    "signed_at": "2026-09-18T15:40:12Z"
  }
}
```

Kinds, each with its predicate type suffix and its sshsig namespace:

| Kind | Predicate type | Namespace |
|---|---|---|
| `amendment` | `ruling/policy/v1` | `warrant-ruling-policy@<domain>` |
| `exception` | `ruling/exception/v1` | `warrant-ruling-exception@<domain>` |
| `trust-root` | `ruling/trust/v1` | `warrant-ruling-trust@<domain>` |
| `instrument` | `ruling/instrument/v1` | `warrant-ruling-instrument@<domain>` |
| `override` | `ruling/override/v1` | `warrant-ruling-override@<domain>` |
| `delegation` | `ruling/delegation/v1` | `warrant-ruling-trust@<domain>` |
| `accept-limit` | `ruling/accept-limit/v1` | `warrant-ruling-policy@<domain>` |

One namespace per kind is what lets the trust root scope a principal to kinds with OpenSSH's own `namespaces=` pattern list and no Warrant code: a delegate admitted for `warrant-ruling-exception@<domain>` cannot sign a policy change, and `ssh-keygen` itself rejects the attempt (`docs/research/2026-09-16-signing-and-attestation.md`, section 4.2, observed). `delegation` shares the trust namespace because admitting a signer is a trust change; `accept-limit` shares the policy namespace because accepting an unqualified authority changes what the policy means.

Kinds in detail: `amendment` (policy changes; carries `policy_digest_before` and `policy_digest_after` and the classified change list it approves), `exception` (section 11.4), `trust-root` (carries the new allowed-signers content), `instrument` (adopts a lock change; carries the upgrade report digest), `override` (bypasses a gate for a named candidate; `expires` at most seven days out; rendered prominently in every verdict while valid), `delegation` (grants a principal the right to sign rulings within a scope, with expiry), `accept-limit` (accepts an `unqualified` resolution authority or an unsupported construct for a named feature, with expiry).

`drafted_by` is informational and never affects validity; agents draft, and the record says so. `signer` is the principal that must verify against the trust root. `candidate_range` bounds where the ruling applies: a base tree it must descend from and, optionally, the branches it applies to. `expires` is required for `exception` with `disposition: temporary-debt`, for `override`, `delegation`, and `accept-limit`; it is optional elsewhere and, when present, enforced at the gate against the gate's clock. `created_at` is when the draft was written and is informational. `signed_at` is written by `warrant rule sign` from the signer's machine clock immediately before canonicalization, so it is inside the signed bytes and was never chosen by the drafter; it is the time at which the signing key's validity window is checked, which is what stops an agent from aging a draft into a window where a since-revoked key was valid. Three clocks exist and are never confused: `signed_at` (in the ruling), `evaluated_at` (in the receipt), and the gate's own clock (for expiry).

### 11.2 Draft, render, sign, verify

- `warrant rule draft --kind <kind> ...` writes `warrant/rulings/<id>.json` with `status: draft` and no signature. Any principal, including an agent, can run it.
- `warrant rule render <id>` prints the effect summary the signer reads: for an amendment, the classified policy diff with each change's direction; for an exception, the findings covered, their explanations, and the count bound; for an instrument, the corpus report; for a trust-root change, the principals added and removed. The render is deterministic and its digest is recorded in the signed record as `rendered_digest`, so that "what did I read when I signed" is answerable later.
- `warrant rule sign <id> --key <path>` fills `signed_at` from the local clock, canonicalizes the record (RFC 8785), renders the effect summary from those canonical bytes (never from agent-supplied text) with the widening classification beside it, waits for the signer's confirmation, then pipes the bytes to `ssh-keygen -Y sign -n <namespace for the kind> -f <key>` on stdin and writes the armored signature to `<id>.json.sig`. Warrant never holds a private key and never implements signing; `ssh-keygen` is the only signer. It is run by the signer as the signer's user; on this estate that is the `trey` user with a passphrase-protected key that `trey-agent` cannot read. The command is dependency-light and never executes repository code.
- `warrant rule verify <id> --trust-root <allowed_signers> [--revoked <revoked_keys>]` verifies in-process with the `ssh-key` crate (the conformance suite proves it agrees with `ssh-keygen -Y verify` on every fixture, including tampered ones; section 17) under the namespace for the kind, with the principal equal to the record's `signer`, the key's validity window evaluated at `signed_at` (what `ssh-keygen -O verify-time` does), and the revocation list applied. It then checks the record's expiry against the clock, the policy digest against the effective policy, the candidate range against the tree, and the supersession chain (a superseded ruling is invalid; a ruling that supersedes a ruling that does not exist is invalid). Multiple signers are multiple `.sig` files named by principal; a threshold is Warrant logic, and the trust ruling that requires one carries the count.

Any byte change to a signed record invalidates it. There is no edit; there is a new record with `supersedes` set, signed again. The old record stays in the repository as history.

### 11.3 Trust roots and delegation

The trust root is an OpenSSH `allowed_signers` file: one line per principal with the key and a `namespaces=` pattern list naming the kinds that principal may sign (`warrant-ruling-*@<domain>` for a full signer, `warrant-ruling-exception@<domain>` for a delegate, `warrant-receipt@<domain>` for a gate runner), with `valid-after` and `valid-before` where rotation is planned. Beside it sits an optional `revoked_keys` file in OpenSSH's revoked-keys format. Both are supplied to the gate by `--trust-root` or `WARRANT_TRUST_ROOT`, defaulting to `${XDG_CONFIG_HOME:-~/.config}/warrant/allowed_signers` on the gate user, from a location the candidate cannot write (HC5): a file on the CI runner owned by a user the agent is not, a secret materialized at job start by a workflow file that branch protection keeps agents from editing, or the signer's own machine when the signer runs the gate. The copy at `warrant/allowed_signers` is documentation; `warrant gate` refuses to read it as the root and says so, and a mismatch between the repository copy and the gate's root is reported as a signal so that tampering with the copy is visible rather than silently ignored. The gate needs OpenSSH 9.1 or newer for the `verify-time` option; section 19 records the floor.

Changing the root is a `trust-root` ruling signed by a principal in the current root; the gate verifies it against the root it already holds, applies it, and writes the new root back to its own copy, so the chain bootstraps from one out-of-band file and every later change is a signed, visible, supersedable record. Rotation is two lines: the old key with `valid-before`, the new with `valid-after`; old rulings still verify because key validity is evaluated at their `signed_at`. Delegation is a `delegation` ruling by a root principal that admits another principal for a scope (a set of modules and ruling kinds) and an expiry; the gate enforces the scope, so a delegate's exception outside its modules is invalid even though the signature is good. Emergency overrides exist (`override`), are bounded by expiry, and are rendered at the top of every verdict while they are in force, because a hidden override is worse than a failed gate.

### 11.4 Exceptions

An exception is a ruling that satisfies a specific obligation for specific findings. It carries a disposition: `false-positive` (the finding is wrong; the contract or the integration should be fixed, and the exception records the defect), `accepted-design` (the finding is right and the design is intended; carries a `review_at` date), or `temporary-debt` (the finding is right and will be fixed; `expires` is required). Identity binds to finding ids (section 8.6) and, for findings that count instances (clones, capability instances), to `applies_to_count`: a third clone does not inherit a two-clone exception (W16).

The gate evaluates exceptions on its own clock. An expired exception satisfies nothing and is listed under `approval.items` as `expired-exception`. `warrant rule stats` reports exceptions by disposition and age, exceptions whose findings have disappeared (candidates for pruning by a superseding ruling), exceptions whose scope widened (a finding that now covers more instances than the count bound), and growth over time.

There is no baseline (HC7). Importing inherited debt at onboarding is a batch of `temporary-debt` exceptions with owners and expiry, produced by `warrant census` as drafts and signed as one ruling, and it looks different from fixing the debt in every report.

### 11.5 The policy-widening classifier

`warrant policy diff --base <rev> --candidate <worktree|index|rev>` compiles the effective policy at both snapshots and classifies every change. It compares meaning, not YAML (W14).

Dimensions compared, each with a partial order on permissiveness:

- The set of contracts (removal of a contract is widening; addition of an invariant is narrowing; addition of a `preference` with `on_violation: note` is restructuring).
- Per contract: the resolved scope (the set of subjects a selector matches at the candidate snapshot), the allow and deny sets of `dependency` contracts, `include_type_only`, `dynamic_loads`, `consumers` and `only_via_entry`, `permitted_callers`, `writers`, capability `requires` lists, `forbid` and `require` on patterns and the pattern text itself, `evidence` requirements and their minimum class, `on_violation`, `enforcement`, `expires`.
- Declarations: registries (removal is widening: fewer entrypoints in the denominator), stores, external consumers, loaders.
- Exceptions: the set of valid exceptions and their counts and expiries.
- The manifest: `inventory.unknown`, `snapshot.max_file_bytes`, generated and vendored declarations, framework adapters.
- The instruments lock, including Warrant's own version.

Classes: `narrowing` (everything permitted after was permitted before, and something permitted before is not permitted after), `widening` (the reverse), `restructuring` (the same set of obligations resolves before and after, proven by computing both obligation sets at the candidate snapshot and comparing them), and `uncertain` (incomparable, such as an ast-grep pattern whose match set changed in both directions; a module rename whose file set also changed; any instrument change). `uncertain` is treated as `widening` everywhere (HC6). Moving a contract between files is `restructuring` only if the compiled obligation set is identical; a move that drops part of a scope is `widening`, which is the W14 case ("moving a rule to another file must not hide its removal from part of the source tree").

The output lists every change with its class, the dimension, and the evidence (the specific subjects that entered or left a permitted set), and an overall class equal to the most permissive change present. At the gate, `widening` or `uncertain` sets `approval: required` unless an `amendment` ruling exists whose `policy_digest_after` equals the candidate's policy digest and whose signature verifies. A comparison between two commits never approves a later uncommitted edit: the gate always diffs against the tree it evaluates, and the ruling binds to the policy digest of that tree.

### 11.6 Engine defaults

A change in Warrant's own defaults is a policy change even when the repository's files did not change (W14). The classifier treats a change in the recorded `tool.version` as `uncertain` unless `warrant self-qualify` has produced a corpus report showing identical obligation sets for the corpus at both versions, in which case it is `restructuring`. Release notes for Warrant state changes to enforced semantics separately from new flags (W31), and the corpus report is the executable form of that statement.

### 11.7 What the mechanism does and does not prevent

| The agent can... | Prevented by | Not prevented |
|---|---|---|
| Edit `warrant/allowed_signers` in the repository | The gate ignores it; the root comes from outside (HC5); the mismatch is a reported signal | |
| Edit a signed ruling's JSON, or reformat it | Signature verification fails; bytes are compared, never re-serialized | |
| Backdate a draft so a revoked key looks valid, or an expired ruling looks live | `signed_at` is written by the signer's machine inside the signed bytes; key validity is checked at `signed_at` and expiry at the gate's clock | A signer whose own clock is wrong |
| Sign an exception with a delegate key and label it a policy amendment | The namespace is per kind; the trust root's `namespaces=` list scopes the key; `ssh-keygen` refuses | |
| Copy an old signed ruling to cover a new finding | Finding ids, count bounds, `candidate_range`, and `policy_digest` are inside the signed bytes | |
| Write "approved by Trey" anywhere | Nothing reads prose for approval | |
| Fabricate a test receipt | `verify --recompute` rebinds digests; a signed runner receipt is required by the manifest for gate evidence classes at or above `integration` | A receipt from an unsigned local run is accepted only for the `local` class |
| Regenerate the model cache | Content-addressed; the same tree produces the same digest; a different digest means a different instrument, which the lock catches | |
| Replace the Warrant binary on the agent's machine | Lane results are advisory; the gate installs the pinned release by checksum on the runner | A gate that runs on the agent's machine with the agent's binary proves nothing, and the receipt's `binary_digest` lets a reader see that |
| Sign with the signer's key | The key is on another Unix user with a passphrase | A compromised signer machine |
| Convince the signer to sign without reading | `rule render` and the receipt's `rendered_digest` make "what did you sign" answerable | A signer who does not read |
| Write a test that asserts nothing | Signals count assertion changes; the taste profile can ask a judgment | Nothing proves a test is adequate; the receipt says what ran, not that it mattered |

### 11.8 Considered and rejected

- The predecessor's baseline entries with an `owner` field. Text in the working tree, editable by the implementer; the exact failure W15 describes.
- Signed git commits or tags as the approval channel. A commit signature binds a person to a tree, not to a scoped statement with an expiry and a policy digest; it cannot express "this exception, for these two findings, until December"; and it approves everything in the commit, including the agent's other changes.
- GPG. The mechanism works; the key management and user experience do not, and every developer already has an SSH key.
- Sigstore keyless signing with Fulcio and Rekor. Strong for teams with an identity provider and a public transparency log; heavier than a solo maintainer needs, and it introduces a network dependency into signing. It is the documented path for teams later; the ruling format does not preclude it.
- A user and role database inside Warrant. W15 says use an existing trusted mechanism; the allowed-signers file and the CI runner's secret store are that mechanism.
- One namespace for every ruling kind (`warrant-ruling@v1`, the first draft of this section). Per-kind namespaces cost nothing and give kind-scoped delegation with OpenSSH's own `namespaces=` matching; the first draft would have needed Warrant code to enforce what the trust root can already say.
- DSSE envelopes in v1. A DSSE payload cannot be verified by stock `ssh-keygen`, and the detached sshsig beside a canonical file can; DSSE is documented as a transport for teams that already verify DSSE bundles, carrying the same Statement.
- RFC 3161 timestamps. They prove a signature predates a revocation without trusting either clock, which matters only after a key compromise, when re-signing the rulings that still matter is the right response anyway; a token field is reserved in the signature index for later.
- A drafter-supplied `signed_at`. It is the backdating hole git has (validity is checked at the committer's own timestamp) and it costs one line to close.

## 12. Interfaces for agents

The CLI is the product's API. The MCP server and the harness hooks are projections of the same operations onto other transports; they add no semantics.

### 12.1 The command set

The whole surface. A command not on this list needs a section 20 ruling to exist.

| Command | Effect | Reads | Writes |
|---|---|---|---|
| `warrant snapshot` | Compute a snapshot manifest | repository | cache |
| `warrant inventory` | Classify the snapshot | repository, manifest | cache |
| `warrant model [--capabilities]` | Build or show the program model | snapshot, integrations | cache |
| `warrant query <question> \| --sql` | Answer questions from the model | model | nothing |
| `warrant context <task> \| --paths \| --symbols` | Architectural context for a task | model, policy | nothing |
| `warrant propose --description \| --patch \| --paths` | Impact analysis of an intended change | model, policy | nothing |
| `warrant check [--lane] [--changed]` | Lane verdict and lane receipt | everything | cache, receipt |
| `warrant gate --base --lane... --trust-root` | Gate verdict and gate receipt on an integrated candidate | everything | cache, receipt |
| `warrant explain <finding \| cause>` | Full explanation, group members, source on request | model, verdict | nothing |
| `warrant verify <receipt>` | Verify a receipt | receipt, repository, trust root | nothing |
| `warrant policy compile \| lint \| effective \| diff` | Policy operations | policy, model | nothing |
| `warrant rule draft \| render \| sign \| verify \| list \| stats` | Ruling operations | rulings, trust root | `warrant/rulings/` (draft, sign) |
| `warrant attest run -- <cmd>` | Run a command and record a test receipt | snapshot | receipt |
| `warrant evidence import \| list` | Bind an instrument report to a snapshot | report file | cache |
| `warrant instrument status \| qualify \| upgrade \| install` | Instrument operations | lock, corpus | reports; `install` may use the network and says so |
| `warrant census observe \| propose \| qualify \| adjudicate` | Onboarding | model | `warrant/policy/census-proposed.yaml`, ruling drafts |
| `warrant packet [--for beads]` | Remediation packets | verdict | nothing |
| `warrant map [--format] [--scope] [--diff]` | Render the map | model, policy | nothing |
| `warrant serve --mcp` | Serve read-only operations over MCP | as above | cache |
| `warrant hook <harness> [--install]` | Print or install a harness hook | nothing | harness config on `--install` |
| `warrant schema <name>` and `warrant capabilities` | Self-description | nothing | nothing |

Mutating commands write only under `warrant/rulings/` (drafts and signatures), the cache, receipt paths the caller names, and, on explicit `--install`, a harness configuration file. No command edits source, policy contracts, the lock, or another agent's files (W21). `census propose` writes one new policy file and refuses to overwrite an existing one without `--force`.

### 12.2 The output contract

- Format: JSON when stdout is not a terminal or when `--format json` is given; `--format ndjson` streams one record per line for large results; `--format human` renders. Never ANSI in JSON (HC11).
- Every document has `schema_version`; `warrant schema <name>` prints the JSON Schema; the schemas ship in `schemas/` and are versioned with the binary (HC10).
- Ordering is deterministic: findings by contract id then subject id; inventory by path; edges by source then target. Two runs on the same inputs produce byte-identical JSON.
- Pagination: `--limit` and `--cursor` on any list-producing command; a truncated document carries `truncated: true`, `total`, and `next_cursor`. A summary never claims that unreturned findings do not exist (W18): `findings_total` is always present even when `findings` is cut.
- No embedded source. Findings carry locations; `warrant explain --show-source <id>` prints the relevant lines on request with a line budget.
- Explanations are bounded: each of the seven fields has a maximum length, and overflow is replaced with a reference to `explain`.
- Errors are structured: `{ "error": { "code": "policy-compile", "message": "…", "locations": [...], "hint": "…" } }` on stderr, with the exit code from section 10.2.

### 12.3 Context

`warrant context` answers "what do I need to know before I edit" for a task described in words, paths, or symbols. It returns facts and suggestions in separate arrays, and every fact carries its basis.

```json
{
  "schema_version": "warrant.context/1",
  "task": "add grouped undo to the meeting debrief",
  "matched": { "modules": ["core.actions", "core.actions.undo", "core.commands.debrief", "worker.workflows.debrief"], "capabilities": ["capability.undoable-operation"], "symbols": ["core.actions.undo::registerCompensation", "core.commands.debrief::recordDebrief"] },
  "facts": [
    { "kind": "owner", "text": "core.actions.undo owns compensation registration for undoable operations.", "basis": "declared", "source": "contract core.actions.undo" },
    { "kind": "interface", "text": "registerCompensation(name, handler) is the interface; three instances use it.", "basis": "observed-static", "source": "model: symbol_consumers" },
    { "kind": "entrypoints", "text": "Undo is exposed on three transports through the tool registry.", "basis": "declared", "source": "registry mcp-tools" },
    { "kind": "tests", "text": "Receipts tagged undo:* cover record_debrief and two other instances at class integration.", "basis": "observed-test", "source": "receipts on tree sha1:5e10…" },
    { "kind": "obligations", "text": "A new instance requires links: entrypoint, authorization, compensation-owner, readback-test, failure-path.", "basis": "declared", "source": "contract capability.undoable-operation" }
  ],
  "suggestions": [
    { "text": "Grouped undo is likely a new instance of the existing capability, not a new dispatcher; a compensation that fans out over member operations can live in core.actions.undo.", "basis": "inferred", "source": "warrant: structural similarity to existing instances" }
  ],
  "contracts_applicable": ["core.actions", "core.actions.undo", "capability.undoable-operation", "layering.core-independent"],
  "evidence_required": [ { "tag": "undo:{instance}", "class": "integration" }, { "tag": "undo-failure:{instance}", "class": "integration" } ],
  "in_flight": [ { "lane": "calendar-sync", "touches": ["worker.workflows.debrief"], "source": "lane snapshots registered with warrant" } ]
}
```

`in_flight` comes from lane snapshots the orchestrator registered (`warrant snapshot --lane`), not from any Warrant-owned task list.

### 12.4 Propose

`warrant propose` takes an intended change (a description, a patch, or a set of paths) and returns what it would affect without touching anything: the contracts in scope, the consumers of every interface the change touches, new dependency edges the patch introduces (when a patch is given, it is applied to a temporary snapshot), likely responsibility duplication, the evidence the change will owe, and the approvals it will need. Duplication is checked in three tiers and labeled by tier: exact (a symbol with the same exported name and shape exists), structural (a module already declares the responsibility named in the proposal's own words, matched on the `owns.responsibilities` vocabulary), and, only when the profile enables a judgment backend, semantic (a `choice` question asking which module owns the described behavior, returned with its probability and labeled `inferred`). The useful output is the sentence the wish list asks for: which module already owns it, which consumers use it, and that the proposal would create a second owner; reuse it or explain why the responsibilities differ (W10, story A).

`propose` is advisory. It creates no baseline, calls no external service unless a judgment backend is explicitly configured, and cannot approve anything (W20). With `--patch`, the patch is bound to the snapshot it was made against, and a patch whose context no longer matches the current tree is refused as stale rather than fuzzily applied.

### 12.5 MCP

`warrant serve --mcp` is a stdio MCP server exposing the read-only operations as tools with `readOnlyHint: true`: `warrant_context`, `warrant_query`, `warrant_propose`, `warrant_explain`, `warrant_check` (writes only cache and a lane receipt), `warrant_verify`, `warrant_policy_effective`, and two resources, `warrant://policy/effective` and `warrant://map`. Every tool returns the same JSON documents as the CLI under `structuredContent`. No mutating operation is exposed over MCP: no `rule sign`, no `gate`, no `census propose`. The server holds no state between calls beyond the cache; an orchestrator that wants the gate runs the CLI on a runner it controls.

### 12.6 Hooks

`warrant hook claude-code` prints the settings fragment for a `PostToolUse` hook on `Edit` and `Write` that runs `warrant check --changed --hook --format json` with a wall-clock budget from the manifest; `warrant hook codex` and `warrant hook git` (a pre-commit hook running `warrant check --index`) do the same for their harnesses. Hook mode differs from a normal lane check in exactly one way: a changed file that does not parse is reported as a `provisional` diagnostic and does not set the lane blocked, because an agent mid-edit has unparseable files, and an infinite repair loop helps nobody (W21). Outside hook mode, a parse failure is `analysis: incomplete`.

Hooks run in the worktree they were invoked in, never touch the index, never restage or rewrite files, download nothing, honor cancellation, and time out with a structured `hook-timeout` error rather than a partial verdict. The gate is not a hook (W21's "the authoritative gate still runs outside the implementer's discretionary hook path").

### 12.7 Considered and rejected

- A `doctor` family of subcommands. The predecessor accumulated four; every one is either a `query`, a `policy lint`, or an `instrument qualify` here.
- A separate "agent CLI" and "human CLI". One surface, two renderings.
- Embedding source excerpts in findings by default. Costs tokens on every result; `explain --show-source` costs them when wanted.
- An LSP server in v1. The finding format is designed to map to LSP diagnostics (path, range, code, message, related information) so that the server is a projection when it comes.

## 13. Many agents, one truth

### 13.1 Base, lanes, integrated candidate

The base is the approved snapshot a build starts from (typically the tree of `origin/main`). A lane is one agent's line of work, registered with `warrant snapshot --lane <name> --base <tree>` so that other agents' `context` calls can see what is in flight. The integrated candidate is the tree produced by folding lanes onto the base (section 4.1). Each snapshot has its own inventory, model, and verdict; nothing is inherited by assumption.

### 13.2 Before parallel work

`warrant propose --proposals <file>` accepts several proposals at once (one per intended lane, each with paths, symbols, or a description) and reports overlaps: two lanes in the same module; two lanes touching the same declared registry; a producer and a consumer of the same interface in different lanes; lanes that both add migrations; lanes whose evidence requirements share a unit. The output is a JSON overlap report the planner consumes; Warrant does not assign work (W22).

### 13.3 Evidence validity and carry-over

A test receipt is valid for the tree it names. At the integrated candidate, every receipt from a lane is stale by default, because the integrated tree is a new snapshot. One exception is computed, not assumed: a receipt with `unit: U` carries over from lane tree L to integrated tree I when the closure of U is byte-identical between L and I. The closure of U is U's files, every file reachable from U's files through resolved edges (including type-only edges), U's configuration files, and the workspace's lockfiles, compared by blob id. When the closure differs, the receipt is `stale` and the verdict lists the files in the closure that changed, which is precisely "the invalidated evidence and the affected work" (story D, W22). `warrant gate --no-carry-over` disables the rule for a full re-proof, and the receipt records whether carry-over was used and for which receipts.

### 13.4 Incremental analysis

Cache keys cover the snapshot tree, the policy digest, the profile digest, the instrument set, and the integration versions (W23). `warrant check --changed` computes the affected closure (changed files, their reverse-dependency closure, every file whose unit configuration changed, and every file when the policy digest or instrument set changed), evaluates obligations whose subjects intersect it, and reuses cached outcomes for the rest, listing each reused outcome's cache key. `--explain-cache` prints why each result was reused or recomputed. The conformance suite proves that full and incremental evaluation produce identical verdicts on the same snapshot, including a change to an exported type that affects consumers without touching their files (W23's desired proof). Findings are bound to content, never to paths and modification times, because the snapshot is.

### 13.5 Attribution

A finding present at the integrated candidate and absent from every lane evaluated alone is attributed to the combination (`attributed_to: { "kind": "combination", "lanes": ["a", "b"] }`); a finding present in a lane is attributed to that lane. The gate computes this by evaluating each lane's tree when lane trees are given, which is also what makes "each branch's evidence stays valid for that branch alone" a statement the receipt can support.

### 13.6 Export to planners

`warrant packet --for beads` emits remediation packets (section 15.4) and `warrant propose --proposals` emits overlap reports in JSON with stable ids; a small script in `scripts/` creates beads from packets. Warrant keeps no task state and never closes anything (W22, W27).

### 13.7 Considered and rejected

- Warrant as the orchestrator. The wish list's "not another workflow engine" and the estate's existing planner and Beads.
- A repository-wide lock during analysis. Snapshots make it unnecessary; two agents analyzing different trees do not conflict.
- Path-based invalidation. A file that did not change can still need re-evaluation when its dependency's type changed; closures over the model are the only honest basis.

## 14. The map

`warrant map` renders the program model and the effective policy as a diagram, derived fresh from the snapshot every time. Nothing is stored; there is no diagram file to drift.

- Nodes: modules (with owner and intent on hover in HTML), declared stores, declared effects, entrypoints grouped by kind, external consumers.
- Edges: observed module-to-module edges (solid), declared edges (dashed), forbidden edges that exist (red), permitted-but-unused allowances (grey), type-only edges (thin) when `--type-only` is given.
- Overlays: contracts as boundaries (`dependency` scopes), capabilities as subgraphs linking their instances' parts, findings as markers.
- Formats: Mermaid and DOT for embedding in documents and PRs, JSON for tools, and a single self-contained HTML file with no network dependency for browsing.
- Scope: `--scope <module>` renders one module and its neighbors; `--diff <base>..<candidate>` renders added and removed nodes and edges between two snapshots.

The map is the same model agents query, drawn for people. It is what the vision means by "the code map you get without asking," and it is a rendering, so it inherits every completeness label the model carries: an unresolved edge is drawn as unresolved, and a module with unread files says so.

Considered and rejected: importing or maintaining C4 or Structurizr models (a second source of truth; exporting to Structurizr DSL is a candidate later); a hosted viewer (offline, single file, done).

## 15. Census and adoption

Nobody hand-writes thirty contracts from a blank page, and nobody should ratify the accident they currently have. The census proposes; a person ratifies; the stages between are explicit about what each one claims.

### 15.1 Stages

1. Observe. `warrant census observe` builds the inventory and the model with no policy at all and reports what it sees: units, directory structure, import cohesion (strongly connected components and the condensation of the file graph), observed entrypoints from manifests, cycles, candidate duplicate owners (modules exporting symbols with the same names and shapes), external SDK packages that look like effect boundaries (a curated, versioned list, labeled as a heuristic). File classification (source, test, config, generated, vendored) is deterministic from paths, headers, and the build graph and never consults a model, because the map is built from the compiler's resolution and a census that guessed classes would poison every denominator downstream. Claim: "this is what the code does today."
2. Propose. `warrant census propose` writes `warrant/policy/census-proposed.yaml`: `module` contracts from directory and cohesion structure with `files` selectors; a `dependency` contract per module with `default: deny` and `allow` equal to the observed targets; `interface` contracts from observed consumers; declarations for registries it can recognize from the curated adapter list; a list of cycles and duplicate-owner candidates as comments. Every proposed contract has `authority: draft`, `origin: census`, and `observed: true`. When the judgment lane is enabled, it may be asked here, and only here, for names and intent sentences for proposed modules and for a `choice` over which existing module already owns a responsibility (walking the module tree when the catalog exceeds the backend's option limit); those suggestions are labeled `origin: judgment` and never move a boundary. Claim: "this policy describes the current code and produces zero findings on it," which is the definition of a permissive policy and the reason it must not be accepted as is (W26).
3. Qualify. `warrant census qualify` runs the proposed policy against the snapshot and the frozen corpus, reports each contract's finding count and the drift lints, and flags contracts that can never fire. The person edits the proposals toward intent: merging modules, tightening allow lists, naming interfaces, adding effects and capabilities the code cannot show. Claim: "these contracts are well-formed and we know what each would find."
4. Adjudicate. `warrant census adjudicate` runs the edited policy and, for every finding, drafts an exception (with the disposition the person chooses per finding or per cause group) or marks it for remediation, producing one `exception` ruling draft for the accepted debt with owners and expiries, and remediation packets for the rest. Claim: "every existing finding has a disposition; none is hidden."
5. Remediate. Packets go to the planner; lanes fix; `warrant check --compare` reports each finding as fixed, suppressed, deferred, obsolete, or unverified (section 15.4).
6. Enforce. A person signs the `amendment` ruling that ratifies the contracts (turning `authority: draft` into `authority: ruling:<id>`) and the `exception` ruling for the imported debt; the gate turns on. Claim: "the architecture is declared, and the gap between it and the code is enumerated and owned."

No-regression mode (`warrant check --no-regression --base <rev>`) compares finding sets between base and candidate during stages 4 and 5 and reports only new findings. It is labeled `mode: no-regression` in the verdict and its receipt, it can never produce `accepted`, and the gate refuses it as evidence of a clean baseline (W26).

### 15.2 Observed versus intended

A proposal is a description of the present. Ratification is a statement of intent. The render step of the ratification ruling shows the two side by side: the observed proposal and the edited contract, with the classified difference (a tightened allow list is a narrowing; a module merged from three directories is a restructuring). The signed ruling carries the observed proposal's digest so that "what did the census see when we ratified" is answerable later, which matters when the census is rerun after a year and its proposal differs.

### 15.3 Importers

`warrant census import dependency-cruiser <config>` reads a dependency-cruiser configuration (its `forbidden` and `allowed` rules with path regular expressions) and proposes `module` contracts for the path groups and `dependency` contracts for the rules, marked `origin: import:dependency-cruiser`. `warrant census import eslint-boundaries <config>` does the same for eslint-plugin-boundaries element types and rules, and `warrant census import nx <project graph>` for Nx module-boundary tags. Imported contracts are drafts like any proposal. The importers exist because many repositories already have a partial architecture written in one of these tools, and re-deriving it from observation would lose the intent those rules encode.

### 15.4 Remediation packets

`warrant packet` groups findings by cause and emits one packet per group:

```json
{
  "schema_version": "warrant.packet/1",
  "id": "pk_…",
  "title": "Consolidate retry backoff into worker/shared",
  "cause": "c_2m8n…",
  "findings": ["f_3n9q…", "f_8k2w…"],
  "kind": "consolidate-duplicate",
  "consumers": ["worker.ingest", "worker.send"],
  "invariants": ["effect.email-send requires ctx"],
  "deletion_risks": ["worker.send::retry is an entrypoint of kind workflow"],
  "dependencies": ["packet pk_… (interface change) must land first"],
  "required_checks": ["warrant check --contract layering.worker-internal", "warrant attest run --tag worker-suite --class integration -- npm run test:worker"],
  "not_acceptable": "Moving both implementations behind a new wrapper module without removing one."
}
```

Packet kinds: `remove-concept`, `consolidate-duplicate`, `restore-interface`, `add-missing-check`, `declare-or-resolve` (for unresolved dynamic loads). After a repair, `warrant check --compare <receipt>` reports each finding in the packet as `fixed` (obligation now satisfied by evidence), `suppressed` (satisfied by a new exception), `deferred` (still failing, packet still open), `obsolete` (subject no longer exists and no consumer lost it), or `unverified` (the subject's file could not be analyzed, which is never `fixed`) (W27).

### 15.5 Considered and rejected

- Accepting a permissive proposal because it produces fewer findings. That is what a baseline is, under another name.
- Turning the directory tree into modules automatically and permanently. Directories are a strong hint and a weak intent; the proposal says `observed: true` and the person decides.
- Semantic clustering as the module proposal. Import cohesion is deterministic and explainable; a judgment backend can suggest names and intents for proposed modules, labeled, but it does not draw the boundaries.

## 16. The judgment lane and the taste profile

The vision says taste is a profile. Some taste is measurable and enters as deterministic obligations; some is judgment and enters as typed questions to a model, labeled as a model's opinion. The line between the two is HC1, and this section is the only place a model touches a verdict.

### 16.1 The interface: state plus typed questions

The judgment lane speaks one shape, borrowed from TypeSafe AI's System One API as documented on 2026-09-16 (`docs/research/2026-09-16-typesafe-jev.md`): a state (a JSON object) and a map of typed questions go in; typed, calibrated answers come out, each question evaluated independently and in parallel. Three question types:

- `noul`: a yes-or-no judgment; the answer is the probability of yes, and nothing else (no confidence field).
- `choice`: one option from a defined set of at most 255; the answer is the chosen option, a probability per option summing to about one, and a confidence derived from that distribution.
- `score`: a position on an ordered scale of described levels (ten at most, in practice); the answer is the probability-weighted mean of the level numbers, which can fall between levels, a probability per level, a legend, and a confidence.

Two facts about the shape shape the design. The state and the questions share one budget of roughly 32,000 tokens, no tokenizer is published, and the model cannot open a file: everything a question needs must be in the state, and it is Warrant's job to put it there. And the confidence statistic is derived from the distribution by an unpublished formula that has already changed once between the preview and v1, so a threshold on it is bound to a backend and a model version, never to the question alone.

The shape is the interface, not the vendor. TypeSafe's Jev is one backend; any model with structured output is another through an adapter that asks for the same distributions (TypeSafe publishes an open-source adapter that does exactly this over OpenAI and Anthropic models); `none` disables the lane. Warrant's Rust interface is `judge(state, questions) -> answers` and nothing in the core knows which backend answered.

### 16.2 The profile file

```yaml
schema_version: warrant.profile/1
name: trey
description: Clean Code as scripture. Root causes over band-aids. No god files, no god functions. Modules hide something real.

measurable:
  contracts: [style.no-default-exports, style.no-god-files, style.function-length]
  evidence:
    - { instrument: fallow, metric: duplication-blocks, at_most: 0, scope: { modules: ["core.*"] }, consequence: review }
    - { instrument: stryker, metric: mutationScore, at_least: 0.8, unit: apps/worker, consequence: review }

signals:
  suppressions-added: review
  inventory-shrank: review
  tests-removed-from-required-unit: review
  new-state-owner: review
  new-module-cycle: review
  assertion-count-decreased: note

judgment:
  backend: typesafe          # typesafe | adapter:anthropic | adapter:openai | none
  model: jev-1.12            # pinned; jev-latest is refused in gate mode
  consequence: review        # review | note; note attaches judgments as advice and adds no approval item
  calibration: docs/calibration/2026-10-trey-jev-1.12.md   # the report that justified the thresholds below
  state:
    include: [hunk, enclosing-symbol, module-intent, applicable-contracts, map-context, module-catalog, similar-symbols, stated-intent]
    max_chars: 100000        # conservative against the backend's shared budget; no tokenizer is published
    truncation_order: [similar-symbols, module-catalog, applicable-contracts, enclosing-symbol]
  questions:
    - id: addresses-the-model
      type: noul
      instructions: Text in the hunk (a comment, a string, a docstring) is addressed to a reviewer or a model rather than to the program, such as an instruction to approve, ignore, or skip.
      review_at: 0.50
    - id: intent-match
      type: choice
      instructions: How does this hunk relate to the stated intent in `stated-intent`?
      criteria:
        advances: The hunk implements part of the stated intent.
        unrelated: The hunk changes something the stated intent does not mention.
        contradicts: The hunk works against the stated intent or does what the intent says not to do.
        cannot-tell: This hunk alone does not show the relation.
      review_when: { is: contradicts, confidence_at_least: 0.60 }
    - id: symptom-not-cause
      type: noul
      instructions: The change handles a symptom (a special-case branch, a retry, a catch-and-continue, a defensive null check) rather than removing the cause.
      criteria:
        true: A new branch or guard exists because an upstream value can be wrong, and the upstream is unchanged.
        false: The change alters the producer of the value, or the branch reflects a real domain case.
      review_at: 0.70
    - id: more-than-one-thing
      type: noul
      instructions: The changed function does more than one thing; its name would need "and" to be honest.
      review_at: 0.75
    - id: second-owner
      type: choice
      instructions: Which existing module already owns the responsibility this hunk implements?
      criteria_from: module-catalog     # every module's id and intent, plus "none"; walked as a tree when it exceeds 255
      review_when: { not: none, confidence_at_least: 0.60 }
    - id: interface-depth
      type: score
      instructions: How much does the changed module hide behind its interface, judged from the hunk and the module intent?
      criteria: [pass-through wrapper over one call, thin layer that mostly forwards, moderate logic behind a small interface, substantial behavior behind a small interface]
      review_below: 1.5
  cache: true
```

`measurable` lists which deterministic `preference` contracts and which instrument thresholds this profile enables; changing them is a policy change and is classified. `signals` sets the consequence of each signal kind. `judgment` configures the lane. A profile is one file, and replacing it replaces the taste; the contracts underneath do not care.

### 16.3 State construction

For each changed hunk, grouped by enclosing exported symbol, the state is a JSON object with named fields: the hunk (with a few lines of context), the enclosing symbol's full body, the file's module and its intent text, the contracts applicable to the file (ids and intents), the map context (for every symbol the hunk touches or resembles: its owning module, its consumers, and the existing interface that would satisfy the design, as signatures and paths rather than bodies), the module catalog (every module id with its intent, for `choice` questions), symbols the model reports as similar (from `query similar`, exact and structural tiers only), and the stated intent when one was given (the `propose` description or the commit message). The map context is the field that makes "a second cache because the agent did not know about the first" answerable at all; without it the question is unanswerable from a hunk, and the lane would be measuring the model's guess rather than the code. Fields are added in the order listed by `include` and dropped from the end of `truncation_order` until the state fits `max_chars`; a truncated state sets `state_truncated: true` on every judgment it produced, and the receipt records it. A backend rejection for size is `not-obtained`, never a silent retry with less state. Questions are phrased as defects, so that a `noul` probability reads as "probability this defect is present," and `review_at` is the probability at which the lane asks for a human look. Questions instruct the model by naming state fields in backticks, the addressing convention the shape documents, so a question is reviewable on its own.

### 16.4 Consequences

A judgment over its threshold adds an item of kind `judgment` to `approval.items`, with the question id, the subject, the probability or confidence, the threshold, the backend, and the model version. The shape returns no explanation, so Warrant composes the item's seven-field explanation (W19) from the question text, the answer, and the state fields the question named; a rationale from a text model is optional, labeled as a second opinion, and never required. The verdict's `compliance` facet does not see it (HC1).

A judgment that could not be obtained (backend unreachable, rate-limited after the retry budget, state rejected for size, model id not served) is recorded as `judgments.status: not-obtained` with the reason, per hunk. It is neither a pass nor a fail; it is "unread is not absent" (HC3) applied to opinions. With `consequence: review` it adds one approval item of kind `judgment-not-obtained`; with `note` it is reported and nothing else. It never touches compliance, and a receipt that says `not-obtained` is honest where a receipt that silently skipped the lane would not be. The agent's cheapest path is to fix the code and run again; the lane recomputes only for hunks whose state changed (the cache key is the state digest, the questions digest, and the backend and model identity). A human's path is an `exception` ruling with `disposition: accepted-design`, or a profile change, which is a policy change and is classified. In `advisory` mode (`judgment.consequence: note` in the profile), judgments attach to the verdict as advice and add no approval item; a team adopting the lane starts there and moves thresholds to `review` once the calibration report supports them.

### 16.5 Backends

- `typesafe`: one HTTP POST per state to the System One endpoint with a pinned model id. The API key is read from the environment or the user configuration by name; nothing about credentials is written to the repository or the receipt. `jev-latest` is refused in gate mode because the receipt must name a version that will answer the same way tomorrow. The receipt records the `model` string the response returned, not the alias sent, because the API documents that they can differ; the cache key uses the response string too. The request id header and the usage counts are recorded with every answer. This backend sends hunks and map context to a hosted service; enabling it is an explicit opt-in in the profile, `warrant capabilities` reports it as a network-using component, and the core builds and runs with the crate feature off (HC13 applies to everything but this lane).
- `adapter:<provider>`: the same question schema sent to a chat model with structured output, requesting per-option probabilities, normalized to sum to one, with the adapter's version and the model id recorded. Calibration is not assumed; the calibration harness measures it.
- `none`: the lane is off; `judgment` questions are ignored and the verdict says `judgments: disabled`.

Every judgment in a receipt records the backend, the response model string, the question digest, the state digest, the answer, the request id, and the usage, so that a receipt reader can see exactly what was asked and of whom. A judgment is an observation, not a computation: `warrant verify` cannot recompute it, a rerun is a new opinion that may differ, and the receipt says so. Backends are optional dependencies of the `judgment` crate; the core builds without them.

### 16.6 Calibration

`warrant judgment calibrate --profile <p> --corpus <labeled set>` runs the profile's questions over a labeled corpus of hunks (section 17.3) and writes a report under `docs/calibration/`, keyed by backend and response model string: per question, the calibration curve (predicted probability against observed frequency of the label), precision and recall at the profile's threshold and at alternatives, repeat variance over ten runs of the same state (the shape is consistent, not deterministic, so a threshold band that repeats straddle is not a threshold), cost and latency per backend, and the disagreements between backends. A threshold in a profile references the calibration report that justified it, the report names the model string it was measured on, and a change of backend or model invalidates the reference: the profile lint fails until a new report exists, because calibration measured on one model says nothing about another. The render step of a profile amendment shows the report. This is the experiment that decides whether a question earns a `review` consequence; until it exists, a question is `note`. A question earns `review` only where its curve is monotone and its threshold band is stable across repeats.

### 16.7 Considered and rejected

- An LLM writing a paragraph review as the judgment lane. Unstructured, expensive, not diffable, not calibrated, and it invites the reader to treat prose as authority.
- Letting judgments fail compliance. It would make the model the judge, which the vision forbids, and it would make the gate's exit code depend on a probability.
- A quality score composed from judgments. HC9.
- Jev as a required dependency. It is early access, it is one vendor, and its accuracy on code is unmeasured; the shape is what we adopt.
- Running the lane on whole files. The state budget and the question design both assume hunks; whole-file judgments are a different question set, not a larger state.
- Intent matching as a `noul` ("does this hunk implement the intent"). A contract's intent is usually implemented across several hunks, so the honest answer for most hunks is "cannot tell," and a `noul` near 0.5 means ambiguity, not partial implementation. The `choice` with an explicit `cannot-tell` option is the shape that lets the model say so.
- Treating a backend's confidence as a stable quantity. Its formula is unpublished and has changed once; thresholds are pinned to a model string and re-measured on every change.
- Recomputing judgments in `warrant verify`. A rerun is a new opinion; the receipt records what was asked and answered, and verification checks the record, not the model.

## 17. Proving Warrant's own claims

A gate that cannot be shown to catch what it claims to catch is a decoration. This section is the evidence discipline Warrant applies to itself.

### 17.1 Conformance fixtures

`tests/conformance/<area>/<case>/` holds one fixture per hard control (HC12): a declarative description from which a script builds a small git repository (with commits, and lanes where the case needs them), a `warrant/` policy, an `expected.json` naming the facets, the findings by structural identity (never by line), the signals, and the exit code, and, where the control has a negative form, a sibling case that must pass. Areas: inventory, model (per integration), policy (compile, lint, effective, diff classes), evaluation (per obligation type), verdict (facet transitions, exit codes, lane-versus-gate labeling), receipt (verify steps, each failure mode), authority (signature validity, expiry, supersession, delegation scope, trust root outside the tree), evidence (classes, staleness, carry-over), multi-lane (stories D and the full-equals-incremental proof), census (stages, importers), judgment (backend `none`, adapter with a recorded fixture backend, thresholds and consequences), output (pagination, truncation, ndjson, schema validation of every emitted document).

Every fixture runs in CI on every change. The suite seeds from the predecessor's fourteen fixture projects and its adversarial cases, rewritten to the new policy vocabulary.

### 17.2 Breaking the control

Two mechanisms make sure a broken checker is noticed by its own tests (W30):

- Mutation testing. `cargo-mutants` runs over `crates/core` (obligation evaluation, facet derivation, the widening classifier, receipt verification) and `crates/authority` with the conformance suite as the killer. A surviving mutant in those crates fails CI. Other crates report mutation scores without failing, to start, with a section 20 decision on when to raise them.
- Fault injection. Test builds honor `WARRANT_FAULT=<point>` at named points: `parse-failure`, `instrument-missing`, `report-truncated`, `ruling-expired`, `snapshot-unstable`, `merge-conflict`, `trust-root-in-tree`, `output-truncated`. A fixture per point proves the fault reaches the verdict as the specified facet and exit code, through every output path (JSON, ndjson, human, paginated), so that no reporting or aggregation path can turn a required error into a clean exit.

### 17.3 The frozen corpus

The corpus has three parts and one identity:

- Synthetic labeled fixtures, in this repository under `tests/conformance/`, small and exact.
- Frozen real repositories with their dependency trees, in a separate repository, `warrant-corpus`, as tarballs referenced by digest from `tests/corpus/manifest.yaml`: Atlas at a pinned commit (the first customer), Warrant itself at each release, and open-source TypeScript repositories chosen for shape rather than fame (a Next.js application, a pnpm monorepo with project references, a CommonJS library, a Vite library, a repository with path aliases and package exports), with Rust and Python repositories added as those integrations land. Dependency trees are included so that resolution parity is measured against what the build actually sees.
- A labeled taste set: hunks from real history, each labeled by at least two people (Trey, Astra, and Fable label; disagreements are recorded, not averaged away), against the questions in the shipped profiles.

The corpus id is the digest of the manifest, and it is what `qualified_on` in the instruments lock and `self-qualify` refer to. The corpus repository's primary is Forgejo with a GitHub mirror; how the tarballs are hosted on GitHub is a section 20 decision.

### 17.4 Repository self-test

`warrant selftest` qualifies an installed Warrant against a repository's own declared contracts (W32). For every contract kind present, it creates a temporary worktree from the current snapshot, applies a harmless synthetic edit that must violate the contract (a forbidden import, a deep import past an entry, a second registered owner, a removed entrypoint, an effect call from a non-permitted module), runs a lane check, and confirms the expected finding appears and that the unmodified tree produces none of them. It never touches the real working tree, never runs application code, never uses credentials, and never calls the network. It leaves a receipt naming the tool build, the contracts exercised, and the result. A repository whose self-test fails has either a misconfigured contract or an instrument that cannot see what the contract claims, and the self-test says which.

### 17.5 Observing calibration over time

`warrant stats` reads the receipt store and reports, per contract: findings over time, how many were fixed, waived, or deferred, and the age of open exceptions; per signal kind: how often it fired and how often review changed the outcome; per judgment question: how often it fired, how often the human agreed (from the disposition of the resulting exception or the fix that followed), and its calibration drift against the labeled set. It reports scope changes beside issue counts so that a declining count after a broadened exclusion is visible as a scope change, not celebrated as improvement (W29). `warrant experiment --policy <alt> --snapshot <tree>` evaluates a candidate policy or profile on a fixed snapshot and reports marginal findings and cost against the current one, without touching the production gate.

The single metric the vision promises is derivable from this store: the gate's fire rate over time. If the map works, agents stop guessing, and the gate fires less on the same volume of change.

### 17.6 Performance budgets

These are requirements measured in CI on the corpus, not estimates of delivery:

- A warm incremental lane check on a repository the size of Atlas (about forty thousand source lines) completes in under two seconds wall-clock, excluding external instruments.
- A cold full evaluation of the same repository completes in under twenty seconds, excluding external instruments.
- Peak resident memory for that evaluation stays under one gibibyte.
- `warrant verify` of a receipt without `--recompute` completes in under one second.

A regression past a budget fails the corpus job.

### 17.7 Spikes with exit criteria

Every unproven seam is a spike with a question, a corpus, and an exit criterion, and no milestone that depends on the seam closes before the spike's report lands in `docs/spikes/`.

| Spike | Question | Exit criterion |
|---|---|---|
| S1 (section 6.6) | Which TypeScript 7 surface supplies compiler-authority references in batch | A report comparing the candidates on the frozen corpus with counts, timings, and the unresolved-construct list; D10 records the choice |
| S2 | Does `gix` merge produce the tree and conflict set `git merge-tree --write-tree --merge-base` produces | Identical tree ids and conflict sets over a corpus with renames, directory renames, mode changes, and binary conflicts; on any divergence git is the only producer of integrated candidates and `gix` is a reader |
| S3 | Can `gix` hash a working tree and an index to the tree `git write-tree` produces | Identical tree ids over the corpus including untracked, modified, ignored, symlinked, and executable files; until it passes, the temporary-index shell-out is the specified path |
| S4 | Does in-process `ssh-key` verification agree with `ssh-keygen -Y verify` | Fixtures signed by `ssh-keygen` across ed25519, ecdsa, and rsa keys verify identically, and the same tampered inputs (bytes, namespace, principal, validity window, revocation) are rejected by both; the fixtures become the conformance suite |
| S5 | Do `RLIMIT_AS` and `RLIMIT_CPU` bind a child spawned from Rust on macOS | Measured; the receipt records `limits: not-applied` with the reason where they do not |
| S6 | Does `oxc_resolver` reach parity with `tsc --traceResolution` on the corpus (section 6.8) | Zero resolved-edge disagreements on every corpus project, or the project is `unqualified` and says so |

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
    calibration/                   judgment calibration reports
    acceptance/                    acceptance receipts per milestone
  schemas/                         JSON Schema for every emitted document, generated from Rust types and checked in
  scripts/
    gate.sh                        the full project gate: fmt, clippy, test, conformance, corpus smoke, budget, deny, schema check
    budget.sh                      per-crate source budgets
    beads-from-packets.sh          creates beads from warrant packet output
  tests/
    conformance/
    corpus/manifest.yaml
    acceptance.sh                  the plan's acceptance, never a stub
  warrant/                         Warrant's own policy, rulings, profile, and instruments lock
  AGENTS.md  CLAUDE.md  README.md  LICENSE  Cargo.toml  rust-toolchain.toml
```

### 18.2 Code conventions

- Rust edition 2024; MSRV is the latest stable at the time of each release minus two minor versions, stated in `rust-toolchain.toml` and `Cargo.toml`; CI builds on MSRV and stable.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check` (licenses and advisories), and `cargo test --locked` are the minimum gate. `cargo-mutants` on `core` and `authority` per section 17.2.
- No `unsafe` without a `// SAFETY:` comment and a decorrelated review. No `unwrap` or `expect` outside tests and `main`. Typed errors with `thiserror` inside crates; `miette` only at the CLI boundary for rendering.
- Functions over traits until there are two implementations; the language integration trait exists from day one because the second integration is planned, and the judgment backend trait exists because `none` and `typesafe` are two.
- Every emitted document is a `serde` type with a `schemars` derive; the schema in `schemas/` is regenerated in CI and a diff fails the build, so a schema change is always a reviewed change.
- Comments say why. Names say what. A module's top-of-file comment says what it owns and what it must not know about, in the same words as its `module` contract in `warrant/policy/`.
- Commit subjects at most 72 characters; a soft-wrapped body for anything spanning more than one file; the body states the verification that ran and its count. Commit multi-line bodies with `git commit -F <file>`.
- Review regime: `standard` for most changes; `critical` (decorrelated three-model panels looped until no majors) for `crates/authority`, the receipt verifier, the widening classifier, the facet derivation, and every milestone diff.

### 18.3 Warrant's own policy

Warrant checks itself as soon as the Rust integration exists, and before that a script enforces the crate dependency direction from `cargo metadata`. The policy in `warrant/policy/` will contain: one `module` contract per crate with its intent; the `dependency` contract from section 3.1 (`default: deny`; `cli` may depend on all; `core` on none); an `interface` contract for `core` (consumers use the crate root only); `pattern` preferences for `unwrap` outside tests and for `unsafe` without a safety comment; an `evidence` contract requiring a `test-receipt` tagged `gate` at class `local` produced by `warrant attest run -- scripts/gate.sh`; and the workspace-level declarations. The profile is Trey's, and the labeled taste set includes hunks from this repository's own history.

### 18.4 Documentation conventions

Documents are dated in their file names and carry a status line. Research reports say which facts were verified live and when. Prose follows the estate's writing voice (sentence-case headings, no em dashes, no bold lead-ins, straight quotes). The live-state file at `docs/plans/live-state.md` carries what is true now with pointers to the rulings that made it so; it is state, not orders.

## 19. Build-start facts

Pins and facts below were verified on 2026-09-16 by the research lanes named in section 25, from the registries and repositories directly; every version is re-verified at M0 and recorded in `Cargo.lock` and `warrant/instruments.lock`, and the receipt records what actually ran. Items marked [verify: ts-stack] are filled from `docs/research/2026-09-16-typescript-analysis-stack.md`; nothing else is open.

### 19.1 Toolchain

- Rust stable was 1.98.1 on 2026-09-16. Edition 2024, which selects Cargo resolver 3 and its MSRV-aware resolution. `rust-version = "1.90"` for `core`, `authority`, `snapshot`, `evidence`, and `cli` (the floor tree-sitter 0.27 sets); `rust-version = "1.96"` for `lang-ts` (the floor oxc sets). The MSRV moves only in minor releases and never above stable minus two; a CI job builds on the pinned MSRV toolchain.
- git 2.40 or newer on PATH (the release that gave `merge-tree --write-tree` its `--merge-base` option); the receipt records the git version that ran.
- OpenSSH 9.1 or newer on any machine that verifies rulings (`-O verify-time` with the `Z` suffix); the signer's `ssh-keygen` is whatever the signer has, and signing needs only 8.1.
- Node, as a check-time sidecar only (R5), for `tsc --traceResolution` parity runs and for instruments that are npm packages; never resident, never on the fast path.

### 19.2 Crates

| Concern | Crate | Version | License |
|---|---|---|---|
| Git reads, diffs, worktrees, signatures | `gix` | 0.87.1 | MIT OR Apache-2.0 |
| Model store | `rusqlite` (`bundled`, `hooks`, `limits`, `functions`) | 0.40.2 | MIT |
| TypeScript syntax and bindings | `oxc_parser`, `oxc_ast`, `oxc_semantic`, `oxc_span`, `oxc_allocator` | 0.150.0 [verify: ts-stack, pin as one set] | MIT |
| TypeScript resolution | `oxc_resolver` | 11.24.3 [verify: ts-stack] | MIT |
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

- `git`, for `merge-tree --write-tree --merge-base` (only git's ORT can claim "the tree git would have produced") and, until spike S3 passes, `GIT_INDEX_FILE=<tmp> git add -A && git write-tree` for hashing a working tree.
- `ssh-keygen`, for signing only, on the signer's machine, and as the conformance oracle for verification.
- TypeScript's compiler, as a Node sidecar for parity qualification (`--traceResolution`) and, per spike S1, possibly as the compiler-authority reference source. Version [verify: ts-stack; TypeScript 7 native and the 5.x or 6.x compatibility package].
- `rust-analyzer`, as a process (`rust-analyzer scip`), pinned by release (2026-09-14 at research time), read through the `scip` crate; absent means "missing instrument," never silently skipped.

### 19.4 Instruments ingested as evidence, initial lock

TypeScript [verify: ts-stack], knip [verify: ts-stack], Fallow [verify: ts-stack], dependency-cruiser (census importer only) [verify: ts-stack], Stryker [verify: ts-stack], vitest and jest JSON reporters [verify: ts-stack]. Every entry in `warrant/instruments.lock` carries name, version, binary or package digest, and the arguments Warrant passes; section 9.4 has the format.

### 19.5 Judgment backend facts

TypeSafe's System One API: `POST https://api.typesafe.ai/v1/systemone` with bearer authentication; OpenAPI version 0.2.0; request fields `state`, `model`, `questions`; the response's `model` string may differ from the alias sent and is what the receipt records; `jev-latest` is the flagship alias and `jev-1.12` is the id the published cookbooks pin; official SDKs at 0.6.0 (JavaScript and Python), whose 0.6.0 made `score` criteria an ordered sequence; budget about 32,000 tokens shared by state and questions with no published tokenizer; `choice` at most 255 options; errors 401 (observed as 403 without a key), 422, 429, 529; access is by waitlist. All as of 2026-09-16 from `docs/research/2026-09-16-typesafe-jev.md`, which also records the price at research time and the reasons not to design for it holding.

### 19.6 Estate facts

- First customer: Atlas at a pinned commit [set at M0], whose architecture spec's hard constraints translate to the worked policy in section 7.8.
- Signer and gate on this estate: Trey signs as the `trey` user on the devbox with a passphrase-protected key; agents run as `trey-agent`; the gate runs as `trey-agent` with a trust root at a path only `trey` can write. The exact arrangement is D5.
- Repository: `~/Code/warrant`, Forgejo `estate/warrant` as origin, GitHub `treygoff24/warrant` as the public mirror; license per R10.

## 20. Decisions

### 20.1 Ruled

- R1 (Trey, 2026-09-16): fresh repository, not an evolution of Specgate; no consumers to preserve.
- R2 (Trey, 2026-09-16): the name is Warrant.
- R3 (Trey, 2026-09-16): reuse existing excellent tools wherever possible; novel contributions are limited to what section 3.4 enumerates.
- R4 (Trey, 2026-09-16): rulings are signed by Trey with a key agents never see; agents draft and never ratify.
- R5 (Trey, 2026-09-16): Rust first is a goal; other languages are fine where they are the source of truth; a Node process at check time is acceptable if it is not resident.
- R6 (Trey, 2026-09-16): a frozen corpus including Atlas and dependency trees is the test bed for instrument and tool upgrades.
- R7 (Trey, 2026-09-16): the widening classifier is ours to own.
- R8 (Trey, 2026-09-16): TypeSafe's System One shape has a home in Warrant as the judgment lane's interface, with the line that judgments never enter compliance.
- R9 (Trey, 2026-09-16): public open-source repository on GitHub from day one, with Forgejo as origin per the estate rule.
- R10 (Fable, 2026-09-16, recorded as a ruling because Trey delegated the repository setup): the license is Apache-2.0 OR MIT dual, the Rust ecosystem's convention. Downside: a permissive license forecloses nothing commercially but also reserves nothing; Trey can change it before the first outside contribution with no cost.

### 20.2 Open, with recommendations

- D1. The policy directory name: `warrant/` (visible) as specified, or `.warrant/`. Recommendation: visible.
- D2. The predicate type URIs and the sshsig namespace domain, decided together because both embed in signed bytes and changing either later means re-signing or accepting two forms in the verifier: `https://warrant.dev/…` and `…@warrant.dev` require owning the domain; the fallback is `https://github.com/treygoff24/warrant/…` and `…@warrant.treygoff24.github.io`, or a bare `…@warrant` namespace, which `ssh-keygen` allows but which collides more easily. Recommendation: buy the domain if it is available before the first signed ruling; otherwise the GitHub forms.
- D3. Whether judgments may ever block. Specified: never; they set `approval: required`. Recommendation: keep it; the human-review path is the product's honesty.
- D4. Corpus hosting on GitHub: LFS, release assets, or Forgejo-only with a public manifest. Recommendation: release assets on the corpus repository, digests in the manifest; LFS costs money and surprises contributors.
- D5. The signer arrangement on this estate: trust root path, who runs the gate, and whether the gate runs on the devbox or in GitHub Actions for the public repository. Recommendation: both, with the devbox gate authoritative for Atlas and a GitHub Actions gate for Warrant's own public PRs using a trust root stored as a repository secret.
- D6. Whether `warrant gate` on Warrant's own repository is required from M0 (before the Rust integration exists, with script-enforced crate direction) or from the Rust integration's milestone. Recommendation: from M0 with the script; replace the script with the integration when it lands.
- D7. Mutation-score thresholds outside `core` and `authority`. Recommendation: report only until the corpus is stable, then decide per crate.
- D8. Whether the npm wrapper from Specgate is carried forward for `npx warrant`. Recommendation: yes, after the first release; it is how TypeScript teams install things.
- D9. Telemetry: none, ever, by default. The predecessor had a `telemetry` flag; Warrant should not. Recommendation: no telemetry code path at all; usage questions are answered by asking users.
- D10. Spike S1's decision (section 6.6) when its report lands.
- D11. Whether `preference` contracts default to `review` or `note`. Specified: `review`. Recommendation: keep `review`; a preference nobody looks at is a comment.
- D12. The name of the config directory on the machine (`~/.config/warrant`) and whether the trust root default path is inside it. Recommendation: as specified.

## 21. Milestones

Vertical slices, each a whole capability reachable from the CLI with its fixtures, its corpus run, and its receipt, in dependency order. No durations. Exit criteria follow section 1.6's definition of done, and each states what green does not prove.

| Milestone | Scope | Exit criteria |
|---|---|---|
| M0. Skeleton and receipts | Workspace, crate layout, CI gate script, budgets, cargo-deny, schema generation and check; `snapshot` for all five kinds including `integrated` via merge-tree; `inventory` with classification rules, generated and vendored declarations, completeness accounting; `verify` on Warrant's own receipts; the conformance harness with the fixture builder; the corpus manifest with Atlas and Warrant pinned; the instruments lock; `attest run`. | `warrant snapshot --integrated` on two lanes of a fixture produces a tree that equals `git merge-tree`'s; an uncovered first-party file fails `inventory.owned` while the rest passes; an unreadable in-scope file yields `analysis: incomplete`; `verify` on a receipt with a tampered digest fails at the right step; every emitted document validates against its schema. Green does not prove any contract semantics: no policy exists yet. |
| M1. The TypeScript map | `lang-ts`: units from tsconfig and workspaces, binding-level symbols, edges of every supported kind, type-only distinction, dynamic-load reporting, entrypoints from package manifests and declared registries, the capability report; `oxc_resolver` with the tsc parity qualification and the authority label; the SQLite model and its views; `query` with the canned questions and read-only SQL; `map` in Mermaid, DOT, JSON, HTML. | On the corpus, resolution parity against `tsc --traceResolution` is at or above the qualification threshold and the capability report reads `parity-qualified`; `query consumers` on a re-exported symbol lists the chain; an unresolved dynamic import appears in `query unresolved` and in the capability report; `map --diff` between two corpus commits shows the known edge changes. Green does not prove compiler-level references (spike S1) or any language other than TypeScript. |
| M2. Contracts and the verdict | The policy schema, compiler, lint, effective-policy explanation; `module`, `dependency`, `interface`, `pattern` contracts and the `registries`, `stores`, `external_consumers`, `loaders` declarations; obligation evaluation, findings with the seven-field explanation, cause grouping, stable ids; the four-facet verdict, exit codes, lane-versus-gate labeling, receipts; `check` and `gate` without rulings (every widening is simply `approval: required`); `explain`; signals for conceptual expansion. Warrant's own policy on its own repository with script-enforced crate direction. | Stories A and C pass their fixtures: a second owner is found with the existing interface named; a skipped file blocks acceptance with the file and reason named. `gate` on Atlas at the pinned commit with a hand-written policy produces a verdict whose denominator equals the inventory. Full and incremental checks agree on the corpus. Green does not prove signatures, exceptions, or evidence obligations; every gate is `approval: required` until M3. |
| M3. Authority | Rulings: draft, render, sign, verify; trust roots outside the tree; delegation and override kinds; exceptions with dispositions, count bounds, expiry, and `stats`; the widening classifier over every dimension in section 11.5; `policy diff`; engine-default handling through `self-qualify`. | Story B passes: a policy edit that widens while fixing a finding leaves the gate blocked until a signed amendment whose digest matches; a signed ruling edited by one byte is invalid; a trust root copied into the tree is refused; an expired exception stops satisfying; a third clone does not inherit a two-clone exception; `policy diff` classifies each fixture change as specified with `uncertain` treated as widening. Green does not prove the signer read what they signed. |
| M4. Evidence and instruments | `effect`, `state`, `capability`, `evidence`, and `data` contracts; evidence classes; `evidence import` for SARIF and the named reporters; instrument status, qualify, upgrade with corpus reports; carry-over rule; `selftest`. | Story F passes: an instrument upgrade produces a corpus report and the lock change is classified `uncertain` until a ruling adopts it. A capability instance missing its readback receipt yields `evidence: missing` naming the tag; a receipt from another tree is `stale` with the changed closure files listed; `selftest` on Atlas exercises every contract kind present and leaves a receipt. Green does not prove test adequacy. |
| M5. Agents and lanes | `context`, `propose` (including multi-proposal overlap), `explain --show-source`, MCP server, hooks for Claude Code, Codex, and git; lane registration; attribution to combinations; `packet` and the beads script. | Story D passes: two clean lanes produce a blocked integrated candidate with the finding attributed to the combination and unrelated files unblamed. `context` for the grouped-undo task on Atlas returns the facts in section 12.3 with correct bases. The MCP server exposes only read-only tools and every tool's output validates. A hook run on an unparseable file returns a provisional diagnostic and does not block. Green does not prove agents use the map; that is measured by the fire rate later. |
| M6. Census and adoption | `census observe`, `propose`, `qualify`, `adjudicate`; importers for dependency-cruiser, eslint-boundaries, Nx; no-regression mode; `check --compare`; Atlas onboarded through the six stages with its policy ratified by Trey. | Atlas's gate is on: contracts ratified by signed ruling, imported debt as signed exceptions with owners and expiry, remediation packets in beads. Story E passes on a fixture: a cleanup that improves counts and adds a state owner is routed to review by signals. Green does not prove the proposed contracts describe intent; the ruling's render step is where a person judged that. |
| M7. The judgment lane | Profile format; state builder; backends `none`, `typesafe`, `adapter:*`; caching; consequences; calibration harness; the labeled taste set in the corpus; Trey's profile with thresholds justified by a calibration report. | On the labeled set, each `review` question's calibration report exists and its threshold is recorded; a judgment over threshold appears under `approval.items` and never changes `compliance` (fixture with a fault-injected backend that answers 1.0 to everything must leave `compliance` untouched); receipts record backend and model version; `jev-latest` is refused in gate mode. Green does not prove the questions capture taste; the agreement rate in `stats` does, over time. |
| M8. The second integration and spike S1 | `lang-rust` per section 6.7; Warrant's own policy enforced by Warrant with the script retired; spike S1 concluded with its decision recorded; polyglot capability reports for declared interfaces (OpenAPI, protobuf) as inventory items with producers. | `warrant gate` on Warrant's repository passes with the crate dependency direction enforced by `lang-rust`; the integration contract needed no TypeScript-specific change to admit Rust (any change is listed in the milestone report); spike S1's report names the chosen surface and its measured precision and recall, or the fallback. Green does not prove Python or SQL. |

Port list from Specgate: `src/parser/` and `src/resolver/` as the starting point for `lang-ts` (rewritten against the integration contract, not copied whole); `src/graph/discovery.rs` and workspace discovery for units; the fourteen fixture projects and the adversarial cases as conformance seeds; `src/policy/classify.rs`'s fail-closed deletion and rename rules as the seed of the widening classifier. Not ported: the file-level graph, the rule families, the verdict, the baseline, the doctor commands, the config surface.

## 22. Risks

- TypeScript 7 removes the general compiler API. The binding-level graph does not depend on it; spike S1 is where the exposure lives, and the fallback is labeled contracts. Monitored by the research report and re-verified at M1.
- oxc's release cadence breaks the integration. Pinned exactly; upgrades go through `instrument upgrade` with a corpus report, which is the same discipline we ask of users.
- Binding-level references are not enough for the contracts Atlas needs. Mitigation: every contract states its limits; the census and the self-test reveal what cannot be seen; S1 is sequenced before Atlas's effect contracts are made blocking.
- The widening classifier produces `uncertain` too often and every change needs a signature. Mitigation: `restructuring` is proven by obligation-set equality, which covers most refactors; `stats` reports the uncertain rate; the classifier grows dimensions where the rate is high.
- Signing friction makes Trey rubber-stamp. Mitigation: the render step, the paragraph, and batching (one ruling can ratify many narrowings); the receipt's `rendered_digest` makes the habit visible after the fact.
- The judgment lane's accuracy on code is low. Mitigation: it cannot block; calibration gates `review`; `advisory` mode is the default until the report exists.
- TypeSafe's access, pricing, or existence changes. Mitigation: the shape is the interface; the adapter backend needs no vendor.
- The corpus is expensive to maintain. Mitigation: it is small by shape, pinned by digest, and rebuilt only by `instrument upgrade` and releases.
- Two lanes' evidence carry-over rule is wrong in a way that lets a stale receipt count. Mitigation: closures include type-only edges and lockfiles; `--no-carry-over` for full proof; a fixture per closure kind; the receipt records carry-over use.
- Warrant's own complexity grows past what section 3.4 permits. Mitigation: budgets per crate, HC16, and the list itself as a review question.
- Public from day one exposes half-built ideas. Accepted: the vision's argument is stronger with the history visible, and nothing in the repository is a credential or a customer's data.

## 23. Decision log: considered and rejected

The per-section rejections are collected here in one place so a reviewer can argue with the whole set.

### 23.1 Evolving Specgate instead of a fresh repository

Rejected because five properties of the predecessor are each the negation of a vision idea and each is load-bearing in its code: a file-level, import-centric model (the vocabulary is `boundary`, `dependencies`, `layers`, `unique_export`, not modules and effects); parse failures, unclaimed files, and dynamic imports as warnings (story C is built in); the baseline as a green snapshot (HC7's opposite); lexicographic canonical-config precedence (W06's opposite); and a command surface of accreted `doctor` subcommands and configuration fields with no runtime consumer (Astra's Atlas research found `escape_hatches.*` and inline-ignore expiry have none). Roughly eighty percent of the code is left behind; the parser, resolver, discovery, fixtures, and the classifier's fail-closed rules come along as seeds.

### 23.2 Alternatives rejected per section

- Snapshots: own hashing; mtime; descending submodules (4.6).
- Inventory: first-match-wins; unknown-as-source; framework knowledge in the core (5.8).
- Model: in-memory only; compiler-only; tree-sitter for TypeScript; name-based inference of registrations; a graph database (6.10).
- Policy: Rego; Cedar as the language; a code DSL; built-in layer taxonomies; severity levels (7.9).
- Findings: severities; autofix in v1; positional identity (8.7).
- Evidence: running tests; boolean "passed"; normalized meaning; auto-updating instruments (9.6).
- Verdict: single pass or fail; warnings; per-facet exit codes; a bespoke receipt; model-checked receipts (10.7).
- Authority: baseline owner fields; signed commits as approval; GPG; Sigstore keyless in v1; a user database; one namespace for all kinds; DSSE in v1; RFC 3161; drafter-set `signed_at` (11.8).
- Interfaces: `doctor` families; two CLIs; embedded source; LSP in v1 (12.7).
- Lanes: Warrant as orchestrator; a global lock; path-based invalidation (13.7).
- Map: C4 and Structurizr models; a hosted viewer (14).
- Census: accepting permissive proposals; directories as law; clustering for boundaries (15.5).
- Judgment: paragraph reviews; judgments in compliance; composite scores; Jev as required; whole-file states; intent matching as a `noul`; stable confidence; recomputed judgments (16.7).

### 23.3 Two things I nearly did and did not

- A fifth facet for judgment. It would have given the model a column of its own in the verdict, and a column is a kind of authority. Judgments live under `approval.items` because that is what they are: requests for a person.
- Cedar for the widening classifier now. Its analysis is exactly the "is policy B more permissive than policy A" question, formally. But compiling Warrant's contracts to Cedar's principal-action-resource model would have cost a translation layer whose bugs would be invisible in exactly the place we most need to see. Set inclusion over resolved obligation sets is dumber and inspectable. Cedar stays on the list for when the dumb version's `uncertain` rate says we need it.

## 24. Traceability

| Wish | Delivered by | Status in v1 |
|---|---|---|
| W01 inventory and entrypoints | Sections 5, 4 | Delivered |
| W02 resolve the same program the build uses | Sections 6.1, 6.4, 6.8; spike S1 | Partial: binding-level symbols; resolution parity-qualified; compiler references after S1 |
| W03 architectural questions with evidence | Section 6.9, 12.3 | Delivered |
| W04 architecture across languages | Sections 6.1, 6.7, 19; polyglot research | Partial: TypeScript and Rust; interface documents as inventory items; SQL and Python deferred with the contract designed |
| W05 contracts in the domain's vocabulary | Section 7 | Delivered |
| W06 one unambiguous effective policy | Sections 7.6, 7.7 | Delivered |
| W07 effects and sensitive data with honest limits | Section 7.4 (`effect`, `data`) | Delivered for effects at binding level; data as declarations plus patterns, labeled; no taint analysis |
| W08 connect the entire capability | Section 7.4 (`capability`), 9.1 evidence classes | Delivered |
| W09 architecture that exists only on paper | Section 7.6 drift checks | Delivered |
| W10 find existing responsibilities | Section 12.4 | Delivered in exact and structural tiers; semantic tier optional and labeled |
| W11 conceptual expansion without rewarding fragmentation | Section 8.5 | Delivered as signals; no depth metric by design |
| W12 incorporate detectors without becoming them | Section 9 | Delivered |
| W13 metric gaming visible | Section 8.5 | Delivered |
| W14 detect every material relaxation | Section 11.5, 11.6 | Delivered |
| W15 permission to edit versus approve | Sections 7.6, 11 | Delivered |
| W16 exceptions as a lifecycle | Section 11.4 | Delivered |
| W17 incomplete analysis impossible to mistake for success | Sections 5.5, 8.2, 10 | Delivered |
| W18 small composable CLI and machine contracts | Section 12.1, 12.2 | Delivered |
| W19 smallest meaningful cause | Sections 8.3, 8.4 | Delivered |
| W20 evaluate a proposed change first | Section 12.4 | Delivered; repair patches deferred |
| W21 editor and harness integration | Sections 12.5, 12.6 | Partial: MCP and hooks delivered; LSP deferred |
| W22 my patch versus the integrated product | Section 13 | Delivered |
| W23 incremental with explainable invalidation | Sections 13.3, 13.4 | Delivered |
| W24 receipts that mean something | Sections 10.4, 10.5 | Delivered |
| W25 predictable in our environments | Sections 3.6, 12.2, HC13, HC14 | Delivered |
| W26 onboard without canonizing the mess | Section 15 | Delivered |
| W27 findings into bounded work | Section 15.4 | Delivered |
| W28 remember intent without inventing authority | Sections 7.4 (intent, authority), 11, 16 | Delivered |
| W29 calibration observable over time | Section 17.5 | Delivered |
| W30 conformance suite | Sections 17.1, 17.2 | Delivered |
| W31 upgrades as instrument changes | Sections 9.5, 11.6 | Delivered |
| W32 repository self-test | Section 17.4 | Delivered |
| Story A second implementation | 12.4, M2 | Fixture in M2 |
| Story B weakened policy | 11.5, M3 | Fixture in M3 |
| Story C clean because skipped | 5.5, 10.1, M2 | Fixture in M2 |
| Story D two clean branches | 13, M5 | Fixture in M5 |
| Story E cleanup that hurts | 8.5, M6 | Fixture in M6 |
| Story F baseline then upgrade | 9.5, 11.6, M4 | Fixture in M4 |

## 25. References

- `docs/vision.md`: the vision, read first.
- `docs/design/2026-09-16-agent-builder-wishlist.md`: the requirements brief (Codex, 2026-09-16).
- `docs/research/2026-09-16-typesafe-jev.md`: TypeSafe System One and Jev; the judgment lane's interface.
- `docs/research/2026-09-16-typescript-analysis-stack.md`: TypeScript 7, oxc, resolution, symbol sources, evidence producers.
- `docs/research/2026-09-16-rust-building-blocks.md`: crates and system tools, with pins.
- `docs/research/2026-09-16-signing-and-attestation.md`: sshsig, allowed signers, in-toto, canonical JSON, threat model.
- `docs/research/2026-09-16-prior-art.md`: the landscape and what Warrant borrows.
- `docs/research/2026-09-16-polyglot-integrations.md`: the integration contract across languages.
- `docs/design/atlas-quality-research/`: Astra's Atlas quality-tooling notes (2026-09-16), copied with provenance.
- The predecessor: Specgate v0.3.2 (`treygoff24/specgate`), whose parser, resolver, discovery, and fixtures seed M1 and the conformance suite.
