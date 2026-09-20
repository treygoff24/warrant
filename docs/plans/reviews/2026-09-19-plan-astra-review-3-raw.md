# Astra plan review, round 3 (raw report)

Date: 2026-09-19. Verbatim report from `delegate codex safe --model astra --reasoning-effort low` over the round-2-revised plan, scoped to the eleven round-2 items. Dispositions live in `2026-09-19-plan-astra-review.md`.

I’ll check the eleven reopened items against the patched plan and ratified spec, then look for patch regressions. I won’t write anything to disk.

Most fixes are present. Two concrete gaps remain: M0’s final acceptance now requires item 01, but the plan creates it only in M2; and the verifier still assigns lock and external-artifact checks to receipt bytes alone. I’m finishing the ownership check before giving the verdict.

## Completion report

- Status: completed.
- Reviewed: the implementation plan, ratified spec, both earlier reviews and disposition tables, vision, and live state.
- Verification: inspected `git diff HEAD` and untracked files; parsed all 47 task blocks and traced affected dependencies and file ownership. No unknown dependency IDs. Tracked-diff whitespace check passed; no build or runtime tests run.
- Files changed: none.
- Follow-up: three blocking items remain.

## Round-3 findings

Plan references below are to `docs/plans/2026-09-19-warrant-v1-plan.md`. Spec references are to `docs/specs/2026-09-16-warrant-v1-spec.md`, version 0.3.

| Item | Status | Plan lines | Deciding spec | Assessment |
|---|---|---|---|---|
| R1-1 | Closed | 519, 687, 709 | §§12.3, 17.7 S7, 21 | W1.16 follows W1.4; S7 follows W1.16. M1 checks qualification against the reviewed build and requires a rerun after relevant review fixes. |
| R1-4 | Incomplete, major | 134, 907–916 | §§3.1, 10.5 steps 6–7 | Snapshot, policy, inventory, and model operations now have an injected boundary. But step 6 still runs “on the receipt’s own bytes,” although it must compare against the lock **at the subject tree**. Step 7 likewise needs actual external artifacts, not merely their recorded digests. Neither injected interface supplies those inputs explicitly. |
| R1-6 | Closed | 443–452, 1179 | §§5.4, 6.5, 7.5 | Framework coverage now follows the full corpus, including Next.js. Versioned recognition, qualification, declared loaders, configuration-reference edges, and entrypoints are assigned. |
| R1-8 | Closed as a planning disposition | 54, 68 | §§17.6, 18.1; §20.2 D4 | The missing private CI path is explicitly a blocked prerequisite requiring separate authorization. Local measurements no longer substitute for CI evidence. Atlas CI remains unproved and must not be reported as satisfied. |
| R1-9 | Regressed, major | 142, 309–315, 929–935 | §§17.1–17.2, 18.1–18.2, 21 | Plain acceptance now permits bootstrap, but milestone-final acceptance requires item 01 at M0 and M1. Only W2.10 delivers `01-gate.sh`. M0 therefore cannot close, while W2.10 transitively requires M0 to have closed. |
| R1-12 | Closed | 951, 984–1008 | §§9.4, 17.5 | W2.11 drafts rather than freezes the record. W2.14 finalizes it after review and requalification; W2.C1 requires equality with the actual comparison inputs and invalidates approval on change. |
| R1-13 | Closed | 1036–1052 | §18.2 | W2.R2 reviews the post-comparison delta before R14 and returns changed measured inputs through qualification and approval. |
| R1-15 | Incomplete, major | 136, 388, 429, 449, 523, 990–993 | HC10; §§1.6, 18.2 | Concrete schema ownership, profile registration, fallback fixtures, and mutant-stage ownership are repaired. However, predeclaring modules does not assign their invocation wiring. W1.8 and W1.16 still lack ownership of the existing model/analyzer composition paths, with no explicit earlier task required to wire callable placeholders. W2.14 also now promises a mismatch fixture while owning only reports and the lock. |
| R1-18 | Closed | 227–236, 1071 | §§4.4, 17.2 | The test counts read attempts, asserts zero for index/commit snapshots, and uses a worktree snapshot as a positive control. |
| N1 | Closed | 984–1008 | §§9.4, 12.3, 17.5, 17.7 S7 | Requalification now uses the full comparison policy and requires matching qualification, frozen-input, approval, and comparison identities. Ownership of its new negative fixture remains under R1-15. |
| N2 | Closed | 68, 107, 1175 | §§18.2, 20.2, 21 | The approval range is P1–P13, and the experiment summary agrees with the Sol retry route. |

## Required corrections

- **R1-4:** Supply the subject-tree lock and actual evidence artifacts through explicit inputs or injected operations. Test altered-lock and missing/mismatched-artifact cases through the real CLI composition.
- **R1-9:** Either introduce a bootstrap gate item in M0 and strengthen it in M2, or require only items 02–03 at M0 and 02–05 at M1. Keep 01–10 mandatory at M2.
- **R1-15:** Assign composition-file ownership and order shared writers, or explicitly require earlier tasks to wire callable placeholders. Give W2.14 ownership of its mismatch fixture and the test path that executes it.

The patch-introduced defects are the **major milestone-acceptance deadlock** under R1-9 and the **major unowned mismatch-fixture requirement** under R1-15; neither is counted twice.

Do not approve: R1-4, R1-9, and R1-15 block approval.
