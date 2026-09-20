I’ll trace the M0–M2 tasks against the ratified spec, focusing on evidence contracts, human gates, and the R14 exit. I’ll leave all files and repo state untouched.

The main risks are in the handoffs: several tasks need behavior that their dependencies do not yet deliver, and M2 changes the build after S7 qualifies it without requiring a rerun. I’m also checking the signing and verification close conditions for ways they could pass on weaker evidence than the spec allows.

Status: completed. Reviewed the plan, governing spec, vision, AGENTS.md, live-state, and supplied dirty changes. A read-only dependency trace confirmed missing prerequisites; no build tests or grammar lint were run. No files changed. The plan needs revisions to qualification gates, signing, task handoffs, and verification before approval.

Findings: 2 blocker, 15 major, 2 minor, 1 note.

## Blockers

### 1. Blocker: S7 qualification can fail or become stale before R14

**Plan:** W1.15, W1.G, W2.13, acceptance demos 4–5.  
**Governs:** Spec §§12.3, 17.5, 17.7 S7, 21.

**Defect:** The graph permits progression with an unsuccessful S7 result and never requalifies the changed M2 build used for the outcome comparison.

W1.15 accepts a report marked `met` or `not met` provided the lock agrees with it; W1.G similarly requires quoting the result rather than explicitly requiring success. Separately, M2 changes the executable after qualification. The spec says, “A changed build or retrieval configuration needs an applicable rerun,” and “S7 must pass before the R14 outcome comparison.”

**Fix:** Require successful, applicable S7 qualification at W1.G. Add a qualification-refresh task after the final M2 checker build and before W2.13, binding the actual build, retrieval configuration, reference policy, sample, criteria, and report. Failed qualification must block the comparison, not merely produce an honest report.

### 2. Blocker: M1 can close before S1 is resolved

**Plan:** W1.5, W1.D10, W1.G.  
**Governs:** Spec §§6.6, 17.7 S1, 20.2 D10, 21 M1.

**Defect:** W1.G has neither W1.5 nor W1.D10 among its transitive prerequisites, although resolving S1 is an M1 exit requirement.

Only W2.1 waits for D10; other M2 tasks can start while the supposedly completed M1 still lacks its compiler-reference decision. Moreover, the adoption branch identifies no task that connects the selected spike implementation to production `lang-ts`.

**Fix:** Make W1.G depend on W1.D10. Add a conditional post-D10 task that either integrates and pins the selected reference producer with tested capability reporting, or records and tests the binding-only fallback. A decision to adopt an instrument must not itself enable the `compiler` capability label.

## Majors

### 3. Major: Signing acceptance omits trusted human confirmation

**Plan:** W2.7, acceptance demo 7, its invariant-to-verify rows.  
**Governs:** Spec §§3.7, 11.2; R4.

**Defect:** The signing task can satisfy its close condition by requiring `--key` and invoking SSH without rendering the final signed bytes and obtaining human confirmation.

The spec requires the trusted tool to render effects from the canonical bytes and “ask[] the human to confirm.” Supplying a key path is not that confirmation. Testing that a repository key is ignored also does not prove the signing environment avoids candidate code and configuration.

**Fix:** Add explicit acceptance requirements for render-from-final-bytes, confirmation, refusal/cancellation without signing, trusted executable/configuration selection, and no candidate hooks or plugins. Distinguish synthetic fixture keys from human signing. Preserve the existing D2 prohibition on durable records before identifier selection.

### 4. Major: The authority, verdict, and receipt tasks have circular behavioral dependencies

**Plan:** W2.3, W2.5, W2.7, W2.8, W2.9.  
**Governs:** Spec §§3.1–3.2, 10.1, 10.4–10.5, 11.2–11.5; HC4.

**Defect:** Several tasks promise executable behavior before their producers exist, while later producers already depend on those consumers.

Examples:

- W2.5 implements `check` before W2.9 supplies receipts, although HC4 says, “A verdict without a receipt is not a verdict.”
- W2.5 derives approval without waiting for authority verification or the amendment classifier.
- W2.7 implements receipt-derived exception batches and amendment rendering before findings, receipts, and classification exist.
- W2.9 tests policy-diff acceptance without depending on W2.8.

**Fix:** Split shared record types and pure authority primitives from CLI assembly. Build classification and finding identity next, then wire signing/batches and complete check-plus-receipt output. Make the final assembly depend on every producer. Specify the core data/function boundary so `core` does not acquire forbidden dependencies on `authority`, `snapshot`, or `model`.

### 5. Major: Early model tasks require a CLI command still stubbed downstream

**Plan:** W1.2, W1.3, W1.6, W1.13.  
**Governs:** Spec §§1.6, 6.1, 6.7, 12.6, 21 M1.

**Defect:** W1.2 and W1.6 require model-command behavior that W1.3 implements later, and the delivery adapter consumes a renderer outside its dependency closure.

W1.2 runs command-based model conformance before `model.rs` is implemented. W1.6 requires `warrant model` to expose Cargo units under the same condition. W1.13 consumes `crates/render`, but W1.12 is not its prerequisite; the displayed waves do not impose a runtime barrier.

**Fix:** Separate library tests from command-level model conformance, then add model assembly after both integrations. Alternatively, introduce the partial model command before those acceptance checks. Add W1.12 to W1.13’s blockers.

### 6. Major: M1 lacks producers for parts of its promised map

**Plan:** W0.5, W1.3, W1.9–W1.12, W2.2.  
**Governs:** Spec §§1.7(2), 5.4, 6.5, 7.5, 12.3, 21 M1.

**Defect:** M1 promises registry-aware architectural context and framework entrypoints, but registry recognition first appears in M2 and no task supplies the framework adapters W0.5 assigns to M1.

The plan also leaves model population for declared loaders, external consumers, stores, and effects implicit. Creating database tables and parsing declarations does not connect those declarations to queryable facts. Required-evidence output is an explicit M1 exit requirement but is not an explicit W1.11 deliverable.

**Fix:** Add an M1 declaration-to-model task after policy compilation and language analysis. Move registry recognition needed for the map into it, leaving obligation evaluation in M2. Assign the needed versioned framework adapters and require context fixtures containing actual examples, consumers, and required evidence with their bases.

### 7. Major: The frozen corpus is reduced to Atlas without a scope ruling

**Plan:** W0.8, W1.4, W1.5; Interfaces → Corpus.  
**Governs:** Spec §§17.3, 17.7 S1/S6, 20.1 R6.

**Defect:** W0.8 creates only an Atlas corpus entry, leaving the required real TypeScript repository shapes and their dependency trees without a delivering task.

Section 17.3 names a Next.js application, a project-reference pnpm monorepo, CommonJS and Vite libraries, and path-alias/package-export coverage. Private local storage under P4 is compatible with D4; silently dropping corpus members is a different decision.

**Fix:** Extend corpus preparation with the named shapes, pinned source and dependency artifacts, publishability, and manifest identity. Make S1/S6 consume that complete manifest, reporting any unavailable member as an unresolved prerequisite rather than shrinking the denominator.

### 8. Major: Corpus behavior and applicable performance budgets are not gates

**Plan:** Gate interfaces, W2.9, W2.11.  
**Governs:** Spec §§1.6, 17.6, 18.1, 21.

**Defect:** The gate has no corpus-smoke stage, and W2.11 merely records measurements even when applicable performance budgets are exceeded.

Digest verification proves artifact identity, not behavior on those artifacts. No task enforces the receipt-verification budget either. Deferring incremental analysis does not defer the cold-evaluation, memory, and verification requirements.

**Fix:** Add a corpus stage with semantic expectations and applicable budget failures. Introduce it as the corresponding capabilities become available; include cold evaluation and non-recompute verification at M2. Record measurements and fail on regression rather than treating their presence in prose as success.

### 9. Major: Required checks can disappear while the gate remains green

**Plan:** P5, W0.1, Gate/stages/acceptance interfaces.  
**Governs:** Spec §§17.1–17.2, 18.1–18.2; HC12, HC16.

**Defect:** Discovering only existing executable stages allows deletion or loss of executable permission to remove mandatory checks, while P5 explicitly permits no CI execution as an acceptable fallback.

The same omission affects acceptance items. The plan also does not require the stable/MSRV CI matrix. The spec requires every conformance fixture in CI on every change; local execution is not an equivalent claim.

**Fix:** Assert the required stage and acceptance-item inventory for each milestone, including executable status and nonempty execution. Specify stable and MSRV jobs. Verify an existing CI execution path before W0.G; if none is available, report that prerequisite as blocked. Preserve separate GitHub authorization rather than treating plan approval as permission to push there.

### 10. Major: P6 converts candidate dependencies into mandatory pins

**Plan:** P6, W0.2, Workspace interfaces, W1.4–W1.5.  
**Governs:** Spec §19 opening, §§19.2–19.4; R12.

**Defect:** W0.2 must add every section 19 dependency even though the spec requires fresh compatibility/runtime checks and explicitly makes optional entries nonmandatory.

The governing wording is: “They are candidate pins,” and “Optional entries are not mandatory dependencies.” No M0 task owns the required package, compatibility, licensing, and runtime recheck before locking them.

**Fix:** Replace “every section 19 dependency” with the dependencies needed by the approved slice and qualified choices. Add build-start verification before lock creation, including the actual Git operation floor. Reserve optional SCIP and other deferred dependencies for their adoption branches. P6’s cost includes compatibility and integration rework, not merely unused dependencies.

### 11. Major: Report ingestion lacks report-requirement evaluation

**Plan:** W2.4, W2.6.  
**Governs:** Spec §§7.4, 9.3, 16.2; HC18.

**Defect:** W2.4 specifies receipt matching by class, unit, tag, and scenario, but does not deliver evaluation of report predicates such as absence of findings or metric thresholds.

A Fallow report can be parsed correctly and bound to the correct source while still violating its requirement. Merely establishing that the report exists must not satisfy that obligation.

**Fix:** Explicitly implement scoped report-result predicates and supported measurement thresholds over canonical evidence rows. Add cases for a bound report containing forbidden findings, a failing threshold, and passing controls. Preserve native meanings and pinned metric definitions rather than treating all reports as receipt-presence checks.

### 12. Major: The comparison can freeze an unreviewed or obsolete reference policy

**Plan:** W1.C1, W2.12, W2.C1, W2.11, W2.13.  
**Governs:** Spec §§7.8, 17.5.

**Defect:** W2.C1 can approve the supposedly frozen comparison before W2.11 creates its full contract set, and no checkpoint explicitly requires human review of that complete reference policy.

W1.C1 reviews module, ownership, and alias declarations, not the effects, capabilities, evidence requirements, and consequences later drafted for the comparison. Section 17.5 requires freezing the “human-reviewed reference policy” before running agents.

**Fix:** Let W2.12 draft the protocol early, but finalize its input digests after W2.11 and qualification refresh. Make W2.C1 explicitly approve the full reference policy and final protocol. Any subsequent change to checker identity or frozen inputs must invalidate that freeze.

### 13. Major: M2 has no critical review of the milestone diff

**Plan:** Gates and authority; W2.10–W2.G.  
**Governs:** Spec §18.2.

**Defect:** W2.G reviews the product outcome but does not require the critical, decorrelated review of the complete M2 diff that the spec requires for every milestone.

W2.10’s critical task review is narrower and precedes later changes, including replacement of the dependency-direction gate.

**Fix:** Add an M2 milestone-review checkpoint after all M2 writes, requiring the complete diff at its final HEAD, dispositions, and no remaining majors. Make W2.G depend on both that review and the outcome report. Trey’s usefulness decision does not substitute for the code review.

### 14. Major: P2 loses decorrelation on its documented retry route

**Plan:** P2, Routing, P3.  
**Governs:** Spec §18.2; plan’s stated executor/reviewer independence.

**Defect:** An Opus retry executor can be reviewed and adjudicated by Opus despite the plan claiming that neither role uses the executor’s family.

The fixed reviewer allocation and adjudicator route only support that claim when Astra executes successfully.

**Fix:** Select reviewer and adjudicator pools against the actual executor/fixer family, while retaining three distinct reviewer families for critical work. Park a task when that pool cannot be supplied. Amend P2’s cost-if-wrong to include loss of review independence, not just retry consumption.

### 15. Major: Module and schema ownership leaves required integration writes unassigned

**Plan:** P6, Core module tree, Schema interfaces, W1.11, W2.6, W2.10.  
**Governs:** Spec §§1.5 HC10, 18.2.

**Defect:** Later tasks create modules and published schemas that their ownership declarations do not let them register or check in.

The fixed core tree omits `retrieval` and `fault`, although W1.11 and W2.10 create them and cannot edit `lib.rs`. Later tasks promise policy, context, profile, receipt, and other schemas without owning their generated schema files; the schema stage is specified to reject differences.

**Fix:** Predeclare the missing modules or assign their registration edits explicitly. Give each schema-producing task its generated files and any required generator registration, or assign one dependent integration task that owns those writes. Require tests to prove the new module actually runs, not merely that a filtered test command exits successfully.

### 16. Major: Receipt verification acceptance demands the wrong result

**Plan:** W2.9, acceptance demo 9.  
**Governs:** Spec §10.5, especially steps 4–5 and 9.

**Defect:** The plan expects ordinary verification of a check receipt to return `verified` without requiring recomputation or a producer signature.

The spec says inventory and model bindings remain unverified without recomputation, and an absent producer signature “makes the result `advisory`.” The expected result must follow the inputs, not the desired demonstration.

**Fix:** Split the acceptance cases: unsigned check receipt → `advisory`; signed inspection with unchecked bindings → `verified-with-unchecked`; fully checked signed fixture → `verified`, while remaining non-authoritative as a check receipt. Retain the reformatted-byte failure case.

### 17. Major: M0 promises conformance proofs whose implementations are absent

**Plan:** W0.5–W0.6, W2.1–W2.2.  
**Governs:** Spec §§5.3, 5.7, 17.1, 21 M0/M2.

**Defect:** W0.6 promises section 5.7’s proofs before capability evaluation exists, and no task implements generated-output verification.

Section 5.7 includes a removed registry entrypoint producing a capability finding and generated drift. W0.5 supplies provenance, not `inventory --verify-generated`; registry recognition and capability evaluation arrive in W2.2.

**Fix:** Limit M0 conformance to input/classification behavior available then. Explicitly assign the registry-removal proof to M2. Add generated verification and its drift/absent-input fixtures, with disposable execution and the specified reproducibility semantics, before claiming those inventory capabilities complete.

## Minors

### 18. Minor: Several invariant proofs test weaker properties

**Plan:** Invariant → verify table: W0.3, W0.7, W1.14, W2.5.  
**Governs:** Spec §§4.4, 10.1, 12.2, 17.2; D4.

**Defect:** Some discriminating evidence can remain green when the stated invariant is broken.

Examples:

- Correct staged blob IDs do not prove unrelated worktree bytes were never read.
- Absence of an ESC byte does not prove absence of progress text.
- Absence of home paths and hostnames does not prove absence of private source or transcripts.
- One simultaneous fail/incomplete result does not establish facet independence.

**Fix:** Add read-attempt instrumentation, full machine-output parsing, explicit publication-content review, and a facet-input transition matrix with targeted mutations. Narrow claims where stronger verification is not supplied.

### 19. Minor: Useful in-scope CLI behavior has no owner

**Plan:** W1.7, W2.3–W2.5, W1.13; CLI interfaces.  
**Governs:** Spec §§7.7, 8.3, 12.1, 12.6.

**Defect:** The plan leaves `policy effective`, `evidence list`, execution of emitted focused-check selectors, and changed-source hook diagnostics unassigned or unspecified.

Registering their command families as stubs does not deliver them. In particular, a finding’s suggested `check --contract ... --instance ...` is not useful if W2.5 implements only its enumerated flags.

**Fix:** Assign these operations to their owning tasks with executable fixtures. For diagnostics, add wiring after W2.5 or explicitly carry it to the later adapter milestone; do not imply the startup/prompt adapter supplies post-edit feedback.

## Note

### 20. Note: P1–P11 disposition and cost assessment

**Plan:** Goal lock §4.  
**Governs:** Spec §§17.5–17.7, 18.2, 19, 20.2, 21.

**Observation:** P2, P5, and P6 need substantive reversal; the other rulings are defensible with the qualifications below.

| Ruling | Assessment and concrete disposition |
|---|---|
| **P1** | **Retain.** Ending executable planning at R14 follows §21. A second planning round is an honest consequence; preserve the separate authority needed to change the governing scope. |
| **P2** | **Amend.** Executor choice is an experiment, not a spec conflict. Overturn fixed family routing after fallback. Add the independence risk identified in finding 14. The supplied sources do not independently establish the cited historical model comparison. |
| **P3** | **Retain, complete its implementation.** Critical surfaces match §18.2. Added review cost is honestly stated. Ensure a bounded review ceiling parks unresolved work, and add the missing M2 milestone review. |
| **P4** | **Retain.** D4 makes privacy the governing boundary and hosting secondary. Single-machine availability is an honest limitation, but it cannot waive corpus or CI requirements. Preserve the full corpus membership. |
| **P5** | **Overturn the fallback.** GitHub workflow syntax is a reasonable choice; unexecuted CI is not compliance with §§17.1 and 18.2. State missing CI evidence as a blocker rather than saying local integration checks substitute. |
| **P6** | **Overturn blanket dependency installation.** Skeleton modules and command stubs can reduce collisions, but optional candidate packages must remain conditional. Include compatibility rework and missing registration ownership in the cost assessment. |
| **P7** | **Retain.** Package naming is consistent with the workspace. Mechanical rename cost is honest before published or signed identities depend on those names; qualify it accordingly. |
| **P8** | **Retain.** S4 becoming its fixture suite follows §17.7; deferring macOS measurement is defensible while limits receive no credit. Clarify that S5 deferral does not defer ordinary child-process deadlines, cancellation, or truthful execution reporting. |
| **P9** | **Retain as staged scope.** Two M2 readers and later optional advice fit §21. Correct the rationale: D3 defers model-triggered required review; M5 stages optional advice. Needed type, coverage, or mutation requirements must remain carried, not disappear because their readers are absent. |
| **P10** | **Retain.** The noncontrolled, observational claim is appropriately limited. Record unavailable fields and stalled/abandoned tasks explicitly; otherwise the stated all-task denominator could conceal failures. |
| **P11** | **Retain.** One verified adapter suffices. Expand the cost-if-wrong: unavailable delivery blocks M1/S7 and the R14 experiment; changing adapters requires fresh qualification, not merely a plan amendment. |

I would not approve this draft. With both blockers and the majors fixed, I would approve the bounded M0–M2 plan, subject to a focused recheck of the revised dependencies and acceptance conditions.