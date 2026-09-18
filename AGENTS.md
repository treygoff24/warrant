# Warrant: agent guide

Warrant is a new tool, co-owned by Trey Goff, Fable, and Astra. Read this file, then `docs/vision.md`, then the spec, then `docs/plans/live-state.md`. Nothing else is required to start.

Three layers, by kind: governing documents rule (the vision, the spec once ratified, Trey's rulings, and the estate rules files); procedural documents say how (skills, runbooks); state documents say what is true right now (`docs/plans/live-state.md`, handoffs). Being read first is ordering, not rank. A state document carries facts and pointers; an imperative in one names who ruled it and when, or it is a paraphrase.

## What governs

1. The vision: `docs/vision.md`. The product's argument, its nouns, and the lines it will not cross. Every spec section descends from it; read the two together.
2. The spec: `docs/specs/2026-09-16-warrant-v1-spec.md`, draft 0.3. Under review by Trey, Astra, and Fable; nothing in it is ratified until the live-state file says so. Section 1.7 is the first complete Atlas experience, 1.9 maps the vision's opening examples to supported checks, 1.5 holds the hard constraints, 20 the decisions, and 21 the build sequence including R14's early stop-and-rethink checkpoint. The co-design changes are summarized in `docs/specs/2026-09-17-warrant-v0.3-changes.md`.
3. The live-state file: `docs/plans/live-state.md`. State, not orders. Update it at every session closeout.
4. The work graph: beads. `bd ready` on arrival. House rules are in `CLAUDE.md`.

Research under `docs/research/` and `docs/design/` is the reasoning trail; spec sections 23 and 25 record what was adopted from it. Where a research file disagrees with the spec, the spec governs; where the spec disagrees with the vision, raise it in review rather than picking one.

## Standing orders

- No build without an approved plan. The next unit of work after spec review is `writing-plans` on the ratified spec, into a plan and bead graph in this repository. Planning does not authorize the build; Trey's word does.
- Agents draft rulings; a human signs them. The signing key is never within an agent's reach and no agent asks for it (spec section 11; ruling R4).
- No time estimates in any document (Trey, 2026-09-11). Give order and dependencies instead.
- This repository is public. Nothing committed carries credentials, internal hostnames or addresses, or home-directory paths; third-party text is quoted sparingly and marked as quotation.
- Git. Commits and pushes to `origin` (Forgejo) are ungated at coherent checkpoints. The `github` remote, pull requests, tags, and releases are gated on Trey's explicit word each time. Never amend, force-push, or bypass hooks; stage files by name.
- Commit messages. `type(scope): summary`, subject at most 72 characters, a soft-wrapped body that states what was verified. Multi-line bodies go through `git commit -F <file>`.
- Trey dictates. Read for intent; never nitpick a dictation glitch.

## Conventions the build will follow (spec section 18)

A Cargo workspace under `crates/`; `core` depends on nothing inside the workspace and `cli` depends on everything; Rust edition 2024. Functions over traits until there are two implementations; every emitted document is a `serde` type with a `schemars` derive; comments say why. The workspace, the gate script, and CI do not exist yet. M0 writes them, and this section is replaced when it does.
