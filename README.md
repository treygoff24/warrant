# Warrant

Architecture your agents can read and can't cheat.

Warrant gives coding agents architectural context before they change code: where behavior belongs, which interface to reuse, what rules apply, and what evidence will be required. It then checks the actual candidate against human-approved contracts and reports compliance, analysis coverage, evidence, and pending approval separately. The map reduces guessing; protected checks and human signatures keep agents from quietly relaxing the rules. Whether that guidance improves real agent work is an explicit product test, not an assumed result.

## Status

Specification version 0.3 was ratified by Trey on 2026-09-17 after Astra and Fable reached consensus. There is no implementation plan or code yet. Planning is next; building requires separate approval. The repository retains the vision, earlier spec revisions, and supporting research.

Read in this order:

1. `docs/vision.md`: what Warrant is and the lines it will not cross.
2. `docs/specs/2026-09-16-warrant-v1-spec.md`: the ratified v1 specification, version 0.3. Start with section 1.7 for the complete Atlas experience; `docs/specs/2026-09-17-warrant-v0.3-changes.md` explains the co-design changes in plain language.
3. `docs/research/`: the primary-source research the spec cites.
4. `docs/design/`: the requirements brief from agent builders and the earlier quality-tooling research that fed the spec.

`AGENTS.md` is the guide for agents working in this repository; `docs/plans/live-state.md` says what is true right now.

## Lineage

Warrant is the successor to [Specgate](https://github.com/treygoff24/specgate), rebuilt from a fresh repository rather than evolved in place. The spec's decision log (section 23) says why.

## Authors

Warrant is created by Trey Goff, Fable, and Astra. Fable and Astra are AI collaborators; they co-own the design, and the spec records which of the three made each decision.

## License

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT license (`LICENSE-MIT`), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Warrant by you, as defined in the Apache-2.0 license, is dual licensed as above, without any additional terms or conditions.
