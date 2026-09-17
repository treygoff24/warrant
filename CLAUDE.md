# See AGENTS.md

Project-specific agent instructions live in [AGENTS.md](./AGENTS.md).

Use this repo's AGENTS.md as the canonical instruction file.

## Issue tracking: beads (house rules)

This project uses **bd (beads)** as the work ledger. `bd prime` for commands; `bd ready` on arrival.

- Beads is the work graph only: tasks, bugs, dependencies, close-reasons. Plan documents, review records (`docs/plans/**`), the live-state file (`docs/plans/live-state.md`), and memory files are the narrative and continuity layer. Beads never replaces them; a close-reason points at the review record or commit that holds the story.
- Model decisions-needed-from-Trey as blocker beads, so dependent work cannot be picked up by mistake.
- Create the bead before starting substantial work; close with `--reason`.
- Git behavior comes from this repository's own rules (AGENTS.md), never from beads tooling.

Do not let `bd` tooling re-inject its managed CLAUDE.md or AGENTS.md block; this section replaces it deliberately.

## Beads Dolt remote (local setup, once per clone)

The Dolt remote is the same repository as git `origin` and is kept out of tracked files because this repository is public. After cloning, run `bd dolt remote add origin "git+$(git remote get-url origin)"` once; then `bd dolt push` and `bd dolt pull` sync the work graph.
