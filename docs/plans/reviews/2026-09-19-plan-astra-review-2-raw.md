# Astra plan review, round 2 (raw report)

Date: 2026-09-19. Verbatim report from `delegate codex safe --model astra --reasoning-effort low` over the round-1-revised plan, 371 seconds. Dispositions live in `2026-09-19-plan-astra-review.md`.

I’ll check all 20 dispositions against the revised plan and spec, then trace the changed tasks and dependencies. I’ll keep the review read-only.

The static retry route fixes finding 14’s executor/reviewer family overlap. Finding 6 still conflicts with the spec: it requires adapters for frameworks used by the corpus, while the revision limits them to Atlas. I also found remaining ordering and ownership gaps around S7 and the new reference-producer task.

Completion report:
- Status: completed.
- Reviewed: the revised plan, both round-1 reports, governing spec sections, vision, live state, and dirty/untracked inventory.
- Verification: read-only dependency and ownership checks confirmed the remaining gaps; no syntax lint or build tests run.
- Files changed: none.
- Follow-up: fix the blocking findings below, then re-check the affected dependencies and acceptance conditions.

Round 1 status: 11 closed, 7 incomplete, 1 regressed, 1 disputed.

New findings: 0 blocker, 1 major, 1 minor, 0 note.

## Round-1 dispositions

Severities below describe remaining defects. Closed items are notes and need no further fix. References are to `docs/plans/2026-09-19-warrant-v1-plan.md` and the governing specification.

| # | Status | Severity | Plan task or section | Governing spec | Finding and fix |
|---|---|---|---|---|---|
| 1 | incomplete | blocker | W1.15, W1.16, W1.G | §§12.3, 17.7 S7, 21 | Successful S7 is now mandatory, but it need not measure the final M1 build. Apply the ordering and refresh fix below. |
| 2 | closed | note | W1.D10, W1.16, W1.G | §§6.6, 20.2 D10, 21 | M1 now waits for both the decision and its implementation branch. No further fix to this finding. |
| 3 | closed | note | W2.7; demo 7 | §§3.7, 11.2; R4 | Final-byte rendering, confirmation, refusal, candidate-code exclusion, and synthetic-key boundaries are explicit. No further fix. |
| 4 | incomplete | major | Interfaces; W2.7–W2.9 | §§3.1–3.2, 10.5 | The authority callback repairs one dependency boundary, but the verifier still owns operations it cannot perform through the declared crate dependencies. Specify the remaining boundary below. |
| 5 | closed | note | W1.2, W1.6, W1.13 | §§6.1, 6.7, 12.6 | The initial model command precedes its consumers, and the hook waits for rendering. No further fix. |
| 6 | disputed | major | W1.8; Carried forward | §§5.4, 6.5, 7.5, 21 | Atlas-only framework coverage contradicts the corpus-based requirement, and loaders remain unassigned. Expand the task as detailed below. |
| 7 | closed | note | P12, W0.8 | §§17.3, 17.7 S1/S6 | The named public shapes and dependency artifacts are restored; unavailable members cannot disappear. No further fix. |
| 8 | incomplete | major | P13; W2.9, W2.11 | §§17.6, 18.1 | Atlas budgets are now enforced locally but explicitly skipped in CI when Atlas is unavailable. Restore the required CI proof below. |
| 9 | regressed | major | Gate/acceptance interfaces; W0.1 | §§17.1–17.2, 18.1–18.2 | The unconditional complete-item assertion makes early task verification impossible. Make acceptance requirements stage-specific below. |
| 10 | closed | note | P6, W0.2 | §19; R12 | Dependency selection now follows rechecks and actual scope rather than blanket installation. No further fix. |
| 11 | closed | note | W2.4 | §§9.3, 16.2; HC18 | Report predicates and failing-content controls are explicit. No further fix. |
| 12 | incomplete | major | W2.11, W2.R, W2.14, W2.C1 | §§9.4, 17.5 | Human policy review is restored, but the frozen-input record precedes changes to identities it freezes. Add finalization below. |
| 13 | incomplete | major | W2.R, W2.13, W2.G | §18.2 | The critical milestone review still precedes an M2 code write. Add a final delta review below. |
| 14 | closed | note | P2; Routing | §18.2 | Both executor routes and both fixer routes remain OpenAI; reviewer and adjudicator routes do not. The static-family mechanism fixes the defect. No routing change needed. |
| 15 | incomplete | major | Schema interfaces; W1.8, W1.16; schema-producing tasks | HC10; §§1.6, 18.2 | The prose promises ownership that the task declarations still omit. Assign the concrete paths below. |
| 16 | closed | note | W2.9; demo 9 | §10.5 | Unsigned, signed inspection, and signed recomputation cases now have distinct expected results. No further fix. |
| 17 | closed | note | W0.5–W0.6, W2.1–W2.2 | §§5.3, 5.7, 21 | Generated verification is assigned and later evaluation proofs are removed from M0. No further fix to that allocation. |
| 18 | incomplete | minor | W0.3; invariant table | §§4.4, 17.2 | An unreadable worktree copy does not prove that no read was attempted. Replace that proof as described below. |
| 19 | closed | note | W1.9, W2.4–W2.5; Carried forward | §§7.7, 8.3, 12.1, 12.6, 21 | The missing CLI operations have owners; post-edit diagnostics are explicitly deferred. No further fix. |
| 20 | closed | note | P1–P11 | §§17–21 | The requested ruling qualifications landed; remaining substantive defects are counted under their individual findings. No separate fix. |

## Remaining round-1 defects

### R1-1. Blocker: M1 qualification can still precede its final implementation

**Plan:** W1.16, W1.15, W1.G; lines 519, 687, 708–709.  
**Spec:** §§12.3, 17.7 S7, 21.

**Defect:** W1.15 does not depend on W1.16, so S7 can pass before the reference producer changes the executable, while W1.G requires a passing record without explicitly checking its applicability to the reviewed build.

The read-only dependency trace also confirms that W1.16 is unordered relative to both W1.4 and W1.15, although all three write `warrant/instruments.lock`. Displayed waves do not serialize those tasks.

**Fix:** Order W1.16 after W1.4 and W1.15 after W1.16, preserving existing dependencies. Require W1.G to compare qualification bindings with the final build and configuration, and rerun S7 after any review fix that changes them.

### R1-4. Major: The receipt-verifier boundary remains incomplete

**Plan:** Interfaces, line 134; W2.9.  
**Spec:** §§3.1–3.2, 10.5.

**Defect:** The plan assigns verification steps 1–6 to `core::receipt::verify` “itself,” although tree lookup and inventory/model recomputation belong to crates that `core` cannot depend on.

`RulingVerifier` supplies authority operations, not the missing snapshot, inventory, and model operations.

**Fix:** Define the inputs or injected operations for those checks and assign CLI composition explicitly. Keep the core verifier responsible for checking bindings and deriving results, without duplicating the other crates or adding forbidden dependencies. Require a recomputation fixture that traverses the real composition.

### R1-6. Major: Atlas-only adapters do not satisfy corpus coverage

**Plan:** W1.8, lines 443–452; Carried forward, line 1164.  
**Spec:** §§5.4, 6.5, 7.5, 21.

**Defect:** W1.8 permits declaring that Atlas needs no adapter and defers the rest, despite §5.4 requiring adapters for frameworks the corpus uses and P12 explicitly including a Next.js application.

This is a substantive rejection of the coordinator’s alternative mechanism. The restored corpus makes “Atlas needs none” insufficient. W1.8 also omits declared loaders and their configuration-reference edges.

**Fix:** Select needed adapters from the complete corpus manifest, version and qualify their recognition rules, and test enabled framework entrypoints. Add loader declarations, `config_reference` edges, and `config-referenced` entrypoints with declared provenance.

### R1-8. Major: P13 substitutes local execution for required CI evidence

**Plan:** P13, line 54; W2.9, W2.11.  
**Spec:** §§17.6, 18.1; §20.2 D4.

**Defect:** P13 allows CI to succeed without the Atlas member, although §17.6 expressly requires its applicable performance budgets to be measured in CI.

D4 restricts publication; it does not waive private CI execution.

**Fix:** Name an authorized private CI execution path that can consume Atlas and make its corpus result required. If none exists, retain an explicit blocked prerequisite and obtain any necessary infrastructure authorization separately; do not count local milestone runs as satisfying the CI requirement.

### R1-9. Major: The stronger acceptance inventory prevents bootstrap

**Plan:** Gate/acceptance interfaces, line 142; W0.1, lines 173–174.  
**Spec:** §§17.1–17.2, 18.1–18.2.

**Defect:** `tests/acceptance.sh` must unconditionally require all ten items, Atlas execution, and zero schema stubs, yet W0.1 requires that same script to succeed before any acceptance items exist.

The contradiction persists at later partial stages, including W2.14 before item 10 is created.

**Fix:** Define explicit required-item and implemented-schema inventories for each acceptance stage, with a strict final mode requiring all ten items and all in-scope schemas. Task verification must invoke its declared stage; missing required items must still fail.

### R1-12. Major: The frozen-input record can already be obsolete when approved

**Plan:** W2.11, W2.R, W2.14, W2.C1; lines 951–1008.  
**Spec:** §§9.4, 17.5.

**Defect:** W2.11 freezes checker and lock identities before W2.R may fix the checker and W2.14 necessarily rewrites qualification metadata, with no subsequent owner assigned to refresh the frozen-input record.

Excluding qualification metadata from canonical policy identity does not preserve a frozen digest of the lock file itself.

**Fix:** Assign finalization of `docs/acceptance/2026-M2-frozen-inputs.md` after review fixes and successful requalification. W2.C1 must verify those identities against the actual executable, configuration, lock, corpus, and policy used by W2.13, rather than merely approve the earlier record.

### R1-13. Major: W2.R does not cover the complete milestone diff

**Plan:** W2.R, W2.13, W2.G; lines 972–975, 1023.  
**Spec:** §18.2.

**Defect:** W2.R claims to follow every M2 code write, but W2.13 subsequently creates executable acceptance code in `tests/acceptance.d/10-comparison-report.sh`.

**Fix:** Retain the pre-comparison product-code review, then require a critical review of the remaining milestone delta before W2.G closes. Any resulting change to the measured checker or frozen inputs must return through qualification and approval.

### R1-15. Major: Promised schema and integration ownership is absent

**Plan:** Interfaces, lines 132–140; task `owned_files`, especially W1.7, W1.8, W1.11, W1.16, W2.5–W2.10.  
**Spec:** HC10; §§1.6, 18.2.

**Defect:** Later schema producers still own neither their schema stubs nor generated JSON files, and the new module tasks lack ownership of the registration and composition files needed to make their code reachable.

The ownership scan finds no generated-schema owner after W0.2. The catalog also omits `warrant.profile`, which W2.6 promises. W1.16 promises fallback policy-fixture edits without owning those fixtures; W2.10 adds a stage without owning `scripts/stages/REQUIRED`.

**Fix:** Assign each producer its exact schema stub and generated file, add the profile schema, and assign module registration, CLI composition, fallback fixtures, and required-stage registration explicitly. Order shared-file writers and require nonempty tests that exercise the installed command path.

### R1-18. Minor: The no-worktree-read proof still permits reads

**Plan:** W0.3; invariant table, line 1059.  
**Spec:** §§4.4, 17.2.

**Defect:** An implementation can attempt a worktree read, ignore its permission error, and successfully return the staged blob, passing the proposed test while violating the invariant.

**Fix:** Instrument worktree access and assert zero read attempts for index and commit snapshots. If retaining the permission-based control, independently establish that permission denial actually applies.

## New findings

### N1. Major: M2 requalification uses the wrong reference policy

**Plan:** W2.14, line 984; W2.C1, line 1005; W2.13.  
**Spec:** §§9.4, 12.3, 17.5, 17.7 S7.

**Defect:** W2.14 explicitly retains the original S7 reference-policy digest, while the comparison uses W2.11’s expanded policy containing effects, capabilities, evidence requirements, and consequences.

A passing run under the earlier declarations cannot qualify retrieval under a different policy identity.

**Fix:** Requalify against the exact full reference policy intended for the comparison, preserving the approved sample, criteria, and budget unless separately amended. Record the new policy identity and require equality among the qualification record, frozen-input record, human approval, and comparison inputs. Add a mismatch case that refuses qualification credit.

### N2. Minor: Two summaries retain superseded approval and routing text

**Plan:** “What I need from you to lock,” line 68; “The executor experiment,” line 1160.  
**Spec:** §§18.2, 20.2, 21.

**Defect:** The approval request names only P1–P11 after adding P12–P13, and the experiment summary still names Opus as the fallback despite the corrected Sol routing.

**Fix:** Update the approval range to P1–P13 and make the experiment summary agree with P2 and the executable routing block.

Do not approve as revised; R1-1, R1-4, R1-6, R1-8, R1-9, R1-12, R1-13, R1-15, and N1 block approval.