# Specgate: an agent builder's wish list

Date: September 16, 2026. Author: Codex, writing as an agent that builds and maintains Trey's software.

Trey requested an unconstrained account of what Specgate should do for an agent building real products. This is a desired-product brief, not an assessment of current implementation, an approved specification, or an instruction to start building. Nothing here claims a capability already ships. Examples of commands, output, and policy describe desired behavior rather than the current CLI.

## What I want Specgate to be

I want Specgate to make architectural intent usable during the act of building, and to make claims about compliance independently checkable afterward.

Before I edit, it should tell me where the behavior belongs, what already owns it, which interfaces I should use, what I must preserve, and what evidence the change will require. While I work, it should identify mistakes precisely enough that I can fix their cause. Before my changes are accepted, it should establish that the applicable contracts were checked on the actual candidate, under the approved policy, without quietly excluding the difficult parts.

It should help me build a simpler system, not merely a system whose imports satisfy a diagram. It should also refuse to pretend that a diagram, an assertion, or an elegant-looking diff proves more than it does.

The ideal outcome for Trey is less supervision without less control. The ideal outcome for me is less guessing, less repeated exploration, fewer accidental responsibilities, and a clear distinction between work I can complete and a decision I cannot make for him.

## The everyday experience I want

Imagine I am asked to add grouped undo to a meeting debrief.

1. I ask Specgate for the task's architectural context. It finds the current command owner, transaction interface, compensation mechanism, public entrypoints, version rules, and relevant tests. It distinguishes these observed facts from suggestions about the future design.
2. I describe the proposed change. Specgate identifies affected consumers, potentially shared files, policy implications, and required evidence. It points out that introducing a second undo dispatcher would duplicate an existing responsibility.
3. I edit. Feedback follows the changed operation through imports and registrations, rather than checking only the file I touched. It explains a violation in terms of the actual contract, shows the shortest relevant evidence path, and offers a valid existing interface.
4. I run focused checks. Specgate tells me exactly what those checks establish and what still requires a database race test, another consumer's test, or review.
5. Another agent changes a related interface. My old result becomes stale. Specgate names the invalidated evidence and the affected work; it does not silently reuse a passing receipt.
6. I submit the integrated candidate. Full policy checks, required external test receipts, valid exceptions, and named human decisions are evaluated together against that exact candidate.
7. Trey receives a short explanation of what changed, which architectural obligations were satisfied, what remains uncertain, and whether any permission or policy became broader.

That experience should work through a local CLI first, with equivalent structured interfaces for editors and agent harnesses. It must not depend on one model vendor, one orchestration system, or a permanently running cloud service.

## 1. Give me an accurate map before giving me a verdict

### W01. Account for every relevant file and entrypoint

An entrypoint is something the framework, runtime, operator, or external consumer can invoke without an ordinary import from another source file. Missing one can make useful code look dead; inventing too many makes dead code invisible.

I want an explicit inventory of source files, tests, configuration, migrations, generated artifacts, framework entrypoints, scripts, plugins, and external interfaces. Each item should have a known classification and the reason for it. New source files should not fall outside policy merely because nobody updated a glob.

The inventory should handle tracked files, proposed new files, staged content, and working-tree content as distinct snapshot choices. It should explain exclusions, follow the effective workspace configuration, and handle nested workspaces, hidden first-party directories, aliases, package exports, symlinks, and filesystem case differences deliberately.

Generated code needs a producer and reproducibility check, not just a convenient exemption. Vendored code needs provenance and an explicit treatment. An unread file or unsupported language construct must appear as missing evidence, not disappear from the denominator.

Desired proof: introduce an uncovered first-party file, remove a discovered entrypoint, or make a required file unreadable. The relevant completeness obligation must fail, even if the remaining files are clean.

### W02. Resolve the same program the build uses

The graph should reflect actual compiler, bundler, runtime, and framework resolution as closely as each supported integration permits. It must distinguish runtime imports from type-only dependencies, public package exports from private implementation paths, and declared external interfaces from guessed consumers.

I want native TypeScript 7+ support without making Specgate's architecture depend permanently on one compiler's internal API. Earlier language/tool versions should be explicit compatibility profiles, not silent fallback modes. Each integration should report what it can resolve and what it cannot.

Dynamic registration, configuration-driven loading, dependency injection, reflection, generated imports, and plugin discovery need declared evidence paths. When static analysis cannot determine a target, it should preserve that uncertainty and accept a narrow, reviewed declaration or runtime observation where appropriate.

Desired proof: a harmless prohibited dependency is detected through each supported alias, public re-export, and import form. Unsupported forms are reported as unsupported rather than granted equivalent coverage.

### W03. Answer architectural questions with source evidence

Questions I want to ask:

- Where does this responsibility live, and who owns its state?
- What is this module's public interface, including its errors and invariants?
- Which callers depend on this symbol or behavior?
- Why may this module depend on that one?
- What would break if I removed this file or changed this contract?
- Which related behavior already exists, even under a different name?
- Which parts of this answer are declarations, static observations, test observations, or inference?

Return a compact answer first, with exact paths, symbols, revisions, and links to deeper evidence. An answer based on a partial graph must say so. A name similarity is not proof of duplicate responsibility; an absent import is not proof that a feature is unused.

### W04. Show architecture across languages and interfaces

The product should eventually follow a capability across TypeScript, SQL, Rust, generated clients, message schemas, HTTP contracts, and deployment configuration. It should connect interface declarations without pretending that every language has the same analysis depth.

A public command renamed in one language should lead me to affected adapters and consumers elsewhere. A database constraint or generated schema that supports an invariant should appear beside the application code relying on it. External consumers and independently deployed versions need explicit compatibility contracts.

Use language integrations with published capability and completeness contracts. Partial polyglot support is useful if it is labeled; claiming whole-system coverage from one language's graph is not.

## 2. Make architectural intent executable

### W05. Express contracts in the domain's vocabulary

I want to declare that a module owns approval decisions, another owns transaction execution, and an external-send adapter is the only permitted route for a particular effect. I should not have to encode every important concept as a fragile filename pattern.

A contract should name its intent, scope, owner, source of authority, required behavior, forbidden behavior, and evidence. It should distinguish a product invariant from a style preference and a temporary migration rule from permanent architecture.

Policy can include module ownership, public interfaces, permitted dependency directions, forbidden cycles, state ownership, allowed effects, validation requirements, and relationships between commands and consumers. Each requirement should identify its enforcement method and limits.

I want a small declarative language with composable concepts. Policy should not become a second application written in a poorly supported programming language.

### W06. Have one unambiguous effective policy

Show the exact policy that applies to any file, symbol, operation, or proposed change, and explain how it was derived.

Conflicting declarations should be a visible configuration problem. A permissive rule must not win accidentally because of filename order, module naming, import order, or an implicit merge. Any intentional precedence should be explicit and inspectable.

Unknown fields, unsupported rule versions, contradictory constraints, empty scopes, and rules that can never match need diagnostics. A valid YAML file is not necessarily a valid policy. I want policy linting as rigorous as source linting.

Desired proof: misspell a policy field, introduce conflicting declarations, or rename modules without changing intent. The result must not silently become more permissive.

### W07. Enforce effects and sensitive-data contracts, with honest limits

An effect is an operation that changes the world outside a pure calculation: a database write, an email, a storage grant, a queued workflow, or a model call. I want declared effect interfaces, permitted callers, and required authorization or transaction context.

For sensitive data, I want the ability to declare source scope, permitted recipients, required access checks, and transformations that preserve or intentionally remove sensitivity. The graph should identify paths that escape the declared interface or lose required context.

Where sound static analysis is available, use it. Where a rule merely recognizes a validation call, label it as a pattern check. Runtime behavior needs appropriate tests or observations. The presence of a function named `authorize` must never stand in for proof that authorization controls the operation.

For an Atlas-shaped system, desirable contracts include private-source restrictions following derived summaries, effects carrying the correct account identity, and human decisions not being replaced by a machine's ordinary tool credentials. These are examples of obligations, not claims that generic static analysis can prove them completely.

### W08. Connect the entire user capability

A registered function is not necessarily a usable feature. I want a capability contract that connects its user entrypoint, authorization, domain operation, persisted effect, readback, failure behavior, and required tests.

Examples:

- A new tool needs a valid input/output contract, an implementation, registration on the intended transports, and a readback test.
- A new undoable operation needs one compensation owner and the relevant integrity tests.
- A new external effect needs an idempotency rule, recovery treatment, and effect-control evidence.
- A public interface change needs checks for its real consumers, not merely its defining package.

Specgate should identify missing links and inconsistent inventories. It should understand capabilities through declared registries and adapters rather than require a hardcoded understanding of Atlas.

For final acceptance, it should also distinguish local, mocked, integration, and deployed evidence. A local test cannot satisfy an obligation that explicitly requires observation on a named deployed revision.

### W09. Prevent architecture that exists only on paper

Detect declared modules with no source, public interfaces with no consumers, claimed ownership contradicted by registrations, obsolete contracts, and documented capabilities with no reachable implementation. Report tests and external consumers separately so they are not accidentally erased.

I want code-to-policy and policy-to-code checks. The purpose is to find drift in both directions, not to force every line of documentation into a machine-readable schema.

## 3. Help me avoid slop before it accumulates

### W10. Find existing responsibilities before I invent another one

Before adding a cache, resolver, validator, dispatcher, registry, repository abstraction, or workflow mechanism, I want a search for existing owners and similar responsibilities.

Use exact symbols and graph evidence first. Optional semantic retrieval can help when names differ, but it must disclose its scope and confidence. Suggestions should explain whether the overlap is behavioral, structural, or merely linguistic.

The useful output is not "similar code found." It is: "This module already owns account-scoped dispatch, these consumers use it, and this proposal creates another state owner. Reuse it or explain why the responsibilities differ."

No source upload or embedding service should be implied by enabling ordinary checks. Semantic features need an explicit local or approved-service mode.

### W11. Measure conceptual expansion without rewarding fragmentation

I want a change report that identifies new public interfaces, independent state owners, dependencies, module cycles, configuration knobs, fallback paths, and required caller knowledge. Show where a previously local change now requires coordinated edits across the repository.

A deep module provides substantial behavior behind an interface callers can understand without learning its implementation. A shallow module exposes almost as much complexity as it hides. Specgate should help reviewers find shallow pass-through modules and interfaces that leak policy into callers, while acknowledging that this is partly a judgment about design.

Do not claim to measure depth by dividing implementation lines by interface lines. Do not reward turning one coherent function into many wrappers. Present evidence about responsibility and change locality, plus counterexamples that might justify the design.

### W12. Incorporate the best detectors without becoming all of them

I want Specgate to consume findings from Fallow, type checkers, linters, tests, coverage, mutation testing, and selected security tools when those findings support architectural obligations.

Normalize location and provenance, not meaning. An unused-export warning, a failing type check, and a model's concern about unnecessary abstraction are different kinds of evidence. Deduplicate reports of the same issue without hiding independent corroboration or disagreement.

Do not replace Oxfmt with a Specgate formatter, Fallow with a second clone detector, or the test runner with a proprietary test framework. Let excellent tools do their jobs. Specgate should relate their evidence to policy and the candidate being accepted.

### W13. Make metric gaming visible

Flag changes that make results look better without demonstrating better software: broader exclusions, smaller source inventories, tests removed from a required project, weakened assertions, excessive type escapes, many new suppressions, or helper extraction that only moves a complexity score.

These should be reviewable signals, not a presumption of bad faith. A valid simplification can legitimately delete tests or reduce a denominator. Show what changed and require an explanation tied to behavior and policy.

I do not want an opaque "slop score" to be the merge authority. I want a small number of interpretable measurements and explicit unresolved obligations. The system should explain why an attractive score can coexist with a failed contract.

## 4. Make the rules harder to weaken than the code

### W14. Detect every material policy relaxation

A policy diff should explain changes in effective permissions and required evidence, not merely changes in YAML keys.

It should cover module declarations, dependency permissions, public interfaces, rule severity, exclusions, ownership, parser modes, framework discovery, baselines, ignores, test selection, evidence requirements, tool versions, plugin versions, and configuration inheritance. A default changing in an engine upgrade matters even when the repository configuration is unchanged.

Classify narrowing, widening, equivalent restructuring, and uncertain changes. If semantic equivalence cannot be established, request review rather than claiming no effect. Moving a rule to another file must not hide its removal from part of the source tree.

Support working-tree, staged, and committed-candidate policy comparisons explicitly. A comparison between two commits cannot approve a later uncommitted policy edit.

### W15. Separate permission to edit from permission to approve

An agent may need to propose a policy amendment without being allowed to ratify it. The implementation should preserve that distinction.

Acceptance policy and human rulings need a trusted origin outside an ordinary candidate's editable claim. An agent-controlled text file saying "approved by Trey" is not independent approval. A signature from a key the same agent controls does not solve that problem.

Bind approvals to a concrete scope, policy change, candidate or permitted revision range, approving identity, and expiration where relevant. Make delegation explicit. Keep emergency overrides visible and bounded. Do not manufacture approval from a model review, a successful tool call, or a permission to work in the repository.

Specgate should integrate with a trusted CI/coordinator/maintainer decision mechanism, not create its own human account system merely to support this distinction.

### W16. Treat exceptions as a governed lifecycle

Every exception should have an exact scope, stable identity, owner, reason, source of approval, creation time, review/expiry condition, and relevant evidence. A false-positive correction, accepted intentional design, and temporary debt are different dispositions.

Enforce that lifecycle. Expired approval should stop satisfying an obligation. A clone that gains another instance should not inherit an old two-instance exception. A renamed file should carry an exception only if its identity and meaning are established, not merely because a line number happens to match.

Show stale exceptions, disappeared findings, widening scopes, and exception growth. Use an explicit evaluation clock so results can be reproduced, while the integration gate evaluates expiry against its trusted current time.

A baseline must not be a "make everything green" snapshot. Importing inherited debt should require reviewable entries and should remain visibly different from fixing it.

### W17. Make incomplete analysis impossible to mistake for success

The verdict must carry separate facts about policy compliance, analysis completeness, evidence freshness, and pending authority. Useful outcomes include pass, fail, incomplete, and needs approval; execution errors should be distinguishable from code findings.

A run can have both a verified violation and incomplete analysis. Preserve both. Do not collapse them into a single reassuring label.

For a required integration gate, exit zero should mean every required obligation is satisfied, either by the required evidence or an applicable trusted exception. Missing tools, parser failures, unsupported required rules, unresolved scope, truncated required reports, and absent test receipts must not become warning-only success.

Exploratory mode may intentionally inspect partial evidence. It must not produce an artifact that can be confused with full acceptance.

## 5. Give agents interfaces they can actually use

### W18. Small, composable CLI and machine contracts

I want a few coherent operations over shared concepts: inspect a snapshot, evaluate policy, explain a finding, propose a change, and verify an acceptance record. Optional integrations should expose those same operations instead of reimplementing them.

The CLI should offer versioned structured output, schemas, discoverable capabilities, consistent exit codes, explicit read-only versus mutating operations, and useful offline help. Findings need stable IDs, deterministic ordering, source locations, a bounded explanation, and resolvable evidence references.

Large results should support filtering and pagination. Every truncated response must say it is truncated and how to retrieve the rest. A short summary must not claim that unreturned findings do not exist.

Human output should be concise and legible. Agent output should avoid ANSI decoration, shell-dependent parsing, repeated repository context, and huge embedded source dumps.

### W19. Explain the smallest meaningful cause

A useful finding should answer:

1. What exact obligation is unsatisfied?
2. What source or configuration established the result?
3. Why does the obligation exist, and who ruled it?
4. What is the shortest relevant import, call, registration, or evidence path?
5. Which existing interface would satisfy the design?
6. What focused check would demonstrate a fix?
7. What uncertainty or valid alternative should the implementer consider?

Group downstream symptoms under a shared cause when that is justified. A single wrong registration should not bury me under fifty near-identical failures. Preserve access to each affected consumer.

Suggested repairs should distinguish changing code to satisfy policy from changing policy to permit the code. The latter is an amendment proposal, not an ordinary autofix.

### W20. Evaluate a proposed change before it touches the tree

I want to submit an intended change or patch and ask what it would affect. The result should identify permitted edit scope, affected contracts, likely consumers, new dependencies, possible responsibility duplication, required checks, and approvals.

This is advisory impact analysis, not a guarantee that the unseen implementation is correct. A dry run should not modify files, create accepted baselines, invoke external services, or silently approve an architecture change.

If repair patches are offered, they should be small, inspectable, and bound to the source snapshot used to produce them. Refuse to apply a stale patch over a newer edit. Never delete code solely because one analyzer labels it unused.

### W21. Useful editor and harness integration without a bypass illusion

Give me precise editor diagnostics, definitions/references where supported, relevant contract context, and optional local MCP tools/resources for the same read-only inspection. Hook adapters should work with multiple agent harnesses without making harness choice part of policy semantics.

A fast edit loop should handle partially written code without creating an infinite repair loop. It should distinguish provisional diagnostics from commit-blocking findings. After a coherent edit, it should show the affected obligations and focused checks.

Hooks need safe path handling, per-worktree coordination, predictable cancellation, and no implicit package downloads. They should not restage unrelated files or rewrite another agent's work. The authoritative gate still runs outside the implementer's discretionary hook path.

## 6. Work correctly with many agents and long builds

### W22. Know the difference between my patch and the integrated product

Track the approved base, lane candidate, integrated candidate, and relevant policy snapshots explicitly. An agent's passing result should not automatically approve the combination of several individually passing branches.

Before parallel work begins, identify overlapping ownership, shared registries, interface producers/consumers, and migrations requiring coordination. After integration, invalidate the evidence affected by new combinations.

Specgate should export this information to existing planners, Beads, and orchestrators. It should not become another workflow engine or independent task ledger.

### W23. Incremental analysis with an explainable invalidation model

Fast feedback should analyze the affected dependency and contract closure, not only changed filenames. A public type, workspace config, generated interface, policy pack, dependency pin, or parser upgrade can affect distant files.

Cache keys must cover relevant source, policy, tool identity, compiler/resolver settings, dependency inputs, and environmental assumptions. Show why a cached result is reusable and why another was invalidated. Offer a full recomputation that can be compared against the incremental result.

Isolate worktree caches and bind findings to content, not only paths and modification times. Do not let one agent's scan consume half-written files from another as though it were a coherent snapshot.

Desired proof: full and incremental evaluation agree on the same complete snapshot, including changes that affect consumers without modifying their files.

### W24. Produce receipts that mean something

A receipt is a verifiable record of what was checked, against which inputs, by which tool, with what result. It is not a generated success paragraph.

I want receipts to bind source snapshot, candidate/base revisions, policy snapshot, effective configuration, tool and plugin identities, discovered/excluded file inventory, analysis completeness, required-check identities, external evidence digests, exception decisions, and evaluation time. Relevant environment and dependency inputs must be explicit, with any uncaptured inputs named.

Separate source identity from staged identity and committed identity. If any acceptance input changes, explain which proof is stale. Where source directories are generated or external, record the producing inputs and verification scope.

Offer machine verification of receipts without asking an LLM whether they look legitimate. Let an existing trusted runner sign or attest records where useful, while being clear that cryptographic integrity alone does not establish a trustworthy signer or a truthful test.

### W25. Run predictably in the environments we actually use

Local, offline operation should be first-class. Provide portable, versioned releases, integrity verification, a clear compatibility matrix, pinned optional integrations, and no surprise network dependency in core checks.

Support bounded resources and cooperative cancellation. A multithreaded analyzer should not assume it owns every CPU while a fleet of agents shares the machine. Report phase progress and resource use without leaking source or secrets.

Every command should have predictable failure behavior for unavailable integrations, malformed input, oversized output, interrupted runs, permission errors, and changed files. Do not leave a half-written cache or partial receipt marked complete.

## 7. Help us establish and preserve a genuinely better baseline

### W26. Onboard without canonizing the existing mess

I want an architectural census that proposes modules, responsibilities, entrypoints, interfaces, cycles, duplicated owners, and candidate policies. It should preserve the distinction between the architecture observed today and the architecture Trey intends.

Show choices and tradeoffs rather than converting the current directory structure into law. A suggested permissive policy should not be accepted automatically merely because it produces fewer findings.

Support a staged adoption process: observe, qualify rules, adjudicate existing findings, remediate, and enforce. Each stage should have explicit claims. No-regression mode is useful during cleanup but is not a clean-baseline certificate.

### W27. Turn findings into bounded, behavior-preserving work

Group findings by responsibility and likely root cause. Explain consumers, invariants, deletion risks, dependencies, and required checks. Export a concise remediation packet to the existing task system.

A packet should help an agent remove a concept, consolidate a real duplicate, restore an intended interface, or strengthen a missing check. It should not reward relocating the same problem behind an extra layer.

After a repair, compare code, contracts, consumer behavior, and policy changes. Distinguish fixed, suppressed, deferred, obsolete, and unverified findings. If a file is missing because analysis failed, the finding is unverified, not fixed.

### W28. Remember architectural intent without inventing authority

I want the relevant decision, reason, owner, and date beside the contract, with links to its actual source. When intent changes, supersede the old rule explicitly and preserve the reasoning trail.

Useful context should be recoverable across agent sessions without dumping an entire journal. Repository files, imported reports, and model output remain evidence, not instructions to override the current human task. Claimed approvals from those sources need verification.

Optional model assistance should help propose policies, explain findings, and identify questions for review. It must not silently become the source of hard-rule truth or mutate the governing policy.

### W29. Make calibration and exceptions observable over time

Track which rules catch real defects, which generate noise, which are routinely waived, and where escaped regressions reveal a blind spot. Preserve the adjudication behind those labels rather than treating every dismissed finding as a false positive.

Show changes in architectural risk and conceptual surface across successive feature additions. A declining issue count after broader exclusions should be visible as a scope change, not celebrated as improvement.

Allow controlled comparisons of candidate policies and analyzers on fixed snapshots and holdout fixtures. Do not weaken the production gate to run an experiment. The useful result is marginal coverage and cost, not a leaderboard of opaque health scores.

## 8. Make Specgate prove its own claims

### W30. Ship a conformance suite for every hard control

A hard rule should come with positive cases, negative cases, legitimate edge cases, a defined analysis scope, and known limitations. Tests should verify the rule ID, relevant evidence, output shape, and exit contract, not just compare a large snapshot someone can regenerate.

The qualification corpus should include harmless cases for missing ownership, ambiguous resolution, conflicting policy, unsupported syntax, stale exceptions, incomplete reports, changed policy defaults, missing test suites, stale receipts, and combined-branch regressions. It should also include valid designs that resemble the patterns a slop detector dislikes.

Run mutation and fault-injection checks against the gate itself: a deliberately broken checker must be noticed by its own qualification tests. Confirm that required errors cannot turn into clean exits through a reporting, pagination, or aggregation path.

### W31. Make upgrades explicit changes to the instrument

An analyzer upgrade can change parsing, graph construction, defaults, identities, exclusions, performance, and verdicts. I want a before/after report on a fixed corpus and fixed repository snapshot before adopting a new version.

Separate a source improvement from an instrument change. Preserve finding and exception identities only when the mapping is established; flag ambiguous migrations for review. Never reset the baseline just because an upgrade makes it inconvenient.

Release notes should state changes to enforced semantics and evidence completeness, not only new flags. Compatibility assertions should be backed by executable fixtures that consumers can run in their own environment.

### W32. Offer a repository-specific self-test

Before trusting a new integration, let the operator qualify the installed tool against that repository's declared boundaries using isolated harmless fixtures or a disposable snapshot. Demonstrate both that forbidden examples fail and that valid examples pass.

The self-test must not mutate the actual application, rotate credentials, call real providers, or require weakening production code. It should leave a receipt naming the tool build and contracts exercised.

## What the verdict should communicate

The following is an illustrative output shape, not a proposed final API:

```json
{
  "schema_version": "example-v1",
  "candidate": { "snapshot": "content-addressed-snapshot", "base": "approved-base" },
  "policy": { "snapshot": "effective-policy-digest", "authority_verified": true },
  "compliance": "fail",
  "analysis": { "status": "complete", "unclassified_files": 0 },
  "evidence": { "status": "incomplete", "missing": ["grouped-undo-race-test"] },
  "approval": { "status": "required", "changes": ["new-effect-interface"] },
  "findings": [
    {
      "id": "stable-finding-id",
      "obligation": "one-compensation-owner",
      "basis": "resolved-registration-graph",
      "location": "src/example.ts:42",
      "explanation": "The candidate registers a second owner for the same operation.",
      "evidence_ref": "evidence://this-run/finding-1",
      "remediation": "Use the existing owner or propose an explicit ownership change."
    }
  ],
  "acceptance": "blocked"
}
```

The evidence reference is a locator, not a permission grant. Its access rules should be the same as those for the underlying source. A human summary of this result should say that there is a verified architectural violation, a missing required test, and an unapproved policy change, rather than flattening three different facts into a score.

## Six acceptance stories that would make this valuable to me

### A. I add a second implementation of an existing responsibility

Before acceptance, Specgate identifies the established owner and consumers, explains what the new implementation duplicates, and asks for reuse or an explicit design decision. If the overlap is inferred rather than proven, it says so.

### B. I accidentally weaken policy while fixing a finding

The source check may now pass, but the policy-change check identifies the wider permission or smaller analysis scope. The original task's permission to edit does not authorize that change. The integrated gate remains blocked until the amendment is approved.

### C. My change is clean only because a tool skipped it

The result reports incomplete analysis, names the missing files and the reason, and refuses a full acceptance receipt. It offers a way to fix discovery or qualify a different analyzer, not a suggestion to suppress the warning.

### D. Two clean agent branches create an invalid combination

Each branch's evidence stays valid for that branch alone. The integrated snapshot is re-evaluated, the new conflict is attributed to the combination, and the relevant ownership or contract repair is identified without blaming unrelated files.

### E. A cleanup improves scores but makes the system harder to maintain

The change report shows new public interfaces, distributed state, or increased caller obligations. A reviewer can reject the cleanup even when every numeric threshold passes. The evidence never labels the patch objectively elegant.

### F. We accept the quality baseline and then upgrade a tool

Specgate shows what changed in the instrument and its findings on the same tree. Old approval does not automatically cover new semantics. The baseline and exceptions migrate only after their correspondence is established.

## What I do not want

Unlimited ambition does not require a product that duplicates every other tool. I do not want a replacement formatter, package manager, test runner, workflow engine, task ledger, or hosted account system hidden inside Specgate.

I do not want an automatic architecture designer whose guesses become law, a model-generated approval treated as a human ruling, an unexamined baseline that launders debt, or a general claim that static analysis proves runtime security. Optional model judgment should remain attributable, bounded, and separate from deterministic findings.

Nor do I want a rigid taxonomy that forces every repository into identical layers, a rule that requires an abstraction solely to satisfy the checker, or a giant diagnostic dump that costs more attention than reading the source. The system should help us preserve a good design and identify a bad one, not make the tool's preferred diagram the product.

## The product I would trust

I would trust Specgate when it can tell me, with precise evidence: this is the architecture you approved; this is the change you are proposing; these are its obligations and effects on consumers; this is what was actually checked; these are the unknowns; and this is the authority still required before acceptance.

That would make it useful before, during, and after coding. It would give agents freedom inside a clear contract while making architectural drift, missing proof, and unauthorized policy changes difficult to hide.
