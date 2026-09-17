# Warrant

Architecture your agents can read and can't cheat.

Warrant reads a repository, builds a map of the program from the compiler's own resolution, evaluates the architecture contracts the team has declared against that map, and returns a verdict with a receipt anyone can verify. It exists because most of the code in the repositories it will guard is written by AI agents, and agents need three things a linter does not give them: findings structured enough to act on without guessing, a compliance lane deterministic enough that arguing with it is pointless, and a rule-change path that runs through a human's signature rather than a file the agent can edit.

## Status

Specification under review. There is no code in this repository yet. The first commits are the vision, the v1 spec draft, and the research behind it, so the whole history is public from the first line.

Read in this order:

1. `docs/vision.md`: what Warrant is and the lines it will not cross.
2. `docs/specs/2026-09-16-warrant-v1-spec.md`: the v1 specification, draft 0.2. Start with section 1.7 for the complete Atlas experience; `docs/specs/2026-09-17-warrant-v0.2-changes.md` explains the revision in plain language.
3. `docs/research/`: the primary-source research the spec cites.
4. `docs/design/`: the requirements brief from agent builders and the earlier quality-tooling research that fed the spec.

`AGENTS.md` is the guide for agents working in this repository; `docs/plans/live-state.md` says what is true right now.

## Lineage

Warrant is the successor to [Specgate](https://github.com/treygoff24/specgate), rebuilt from a fresh repository rather than evolved in place. The spec's decision log (section 23) says why.

## Authors

Warrant is created by Trey Goff, Fable, and Astra. Fable and Astra are AI collaborators; they co-own the design, and the spec records which of the three made each decision.

## License

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT license (`LICENSE-MIT`), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Warrant by you, as defined in the Apache-2.0 license, is dual licensed as above, without any additional terms or conditions.
