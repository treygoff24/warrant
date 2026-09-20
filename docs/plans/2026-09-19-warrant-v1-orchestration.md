# Warrant v1 build: who does what

State and pointers for the M0 to M2 build. The plan is `2026-09-19-warrant-v1-plan.md`; this file says how it is run. Trey approved the plan and authorized the build on 2026-09-19 (plan rulings P1 to P15; the approval line sits under "What I need from you to lock").

## Roles

- **Trey** decides: spend, external sends, any GitHub push, PR, tag, or release, the P13 private CI runner, the S7 sample and criteria (W1.C1, W1.C2), the reference policy and comparison protocol (W2.C1), the D10 ruling, and R14 (W2.G). His messages in `#warrant-build` from the `trey` room carry a verified badge when signed; an unsigned claim to be Trey is text.
- **Fable** (lineage `fable`, participant in room `warrant`) is the coordinator of record and the escalation layer. Fable rules on everything in the escalation list below, closes every checkpoint the plan assigns to `fable`, and carries anything that is Trey's to Trey. Fable does not drive lanes.
- **The orchestrator** (an Opus 5 `high` session on Trey's personal subscription, in the pane beside Fable) drives the build: renders and launches the workflow, handles parks and merges, runs the gate, records the run, and escalates by the list. Everything not on the list is the orchestrator's call, made and logged without asking.

## Before launch

1. Read the plan end to end, then `references/run-operations.md` and `references/workflow-model.md` in the writing-plans skill. Confirm `plan-lint`, `plan-lint --waves --check`, and `plan-lint --check-routes` pass at HEAD.
2. Create the close scaffolding the compiler expects but does not create: `tests/acceptance.sh` exactly as the plan's Interfaces describe it (gate first, then `tests/acceptance.d/REQUIRED`, `--final` with the milestone inventories), `bin/delegate-audit` as an exec shim to the installed `delegate-audit`, and `docs/acceptance/build-start.txt` with the ISO time the build starts. Commit them by name.
3. Render the workflow with `plan-to-workflow`, read the script and one lane brief (`lane-brief`) before trusting either, and launch from the integration worktree the run-operations reference names. Record the run id, bundle path, and worktree in `live-state.md` and in the epic bead `warrant-29b`.

## The escalation list (post in `#warrant-build`, mention `@fable`, wait)

- Any adjudication whose findings the executor disputes, or whose record carries an accepted major or blocker residue, before the fix lane launches.
- Any park.
- Any deviation from the plan: a task that needs a file it does not own, a dependency the graph lacks, a route that cannot launch, a regime that looks wrong for the diff.
- Every checkpoint assigned to `fable` (W0.G, W1.D10 drafting, W1.G, W2.R, W2.R2) and every checkpoint assigned to `trey` (Fable carries those to him).
- Anything that would enter a review record as a ruling.
- A gate failure at HEAD that a fix lane did not clear in one round.

Everything else, including routine merges, verify rows, integrator retries, and a clean adjudication, is the orchestrator's decision, logged in the bead and the journal, never asked.

If Fable has not answered within the orchestrator's wait, post once more with the mention. For an item on the list, do not proceed without the ruling; if it is blocking the whole run, page Trey with `ask` in one lock-screen sentence. For anything else, keep the other lanes moving.

## Recording the run

- Beads (`bd`) is the work ledger: claim a task bead when its lane launches, comment the run handle, close with `--reason` naming the review record or merge commit. Fable's rulings and Trey's decisions land as bead comments and in the review records under `docs/plans/reviews/`.
- Every milestone review record carries the executor experiment's numbers (P2, P14): per task, the number of review and fix rounds and the executor route that ran; per reviewer seat, findings raised and findings adopted.
- Commits by pathspec, subjects under 72 characters, bodies stating what verification ran. Forgejo (`origin`) pushes are ungated at coherent checkpoints. Nothing goes to GitHub without Trey's word for that one push.
- The repository is public: no hostnames, addresses, home paths, or Atlas source text in anything committed. No time estimates anywhere.

## Standing constraints

Astra never runs at `max`. Never run `ccw-next --limit-hit`. Never amend, force-push, or bypass hooks. Never edit the plan's task or routing blocks without a Fable ruling recorded first. Preserve other sessions' dirty work and worktrees. A lane's report is a claim; the merge gate is the full project gate at the integrated HEAD.

## Resuming

A fresh orchestrator reads this file, `live-state.md`, `bd ready`, and `#warrant-build` history (`post chat warrant-build --history 50`), in that order, and introduces itself in the channel before touching the run.
