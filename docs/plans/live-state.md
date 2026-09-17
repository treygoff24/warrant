# Warrant live state

State, not orders. The first block is what is true right now, with pointers to the rulings that made it so. Update at every session closeout; earlier blocks are dated history.

REPOSITORY CREATED (2026-09-17). Warrant is the successor to Specgate, renamed by Trey on 2026-09-16 (spec ruling R2) and rebuilt from a fresh repository (R1). This repository holds the vision (`docs/vision.md`), the v1 spec draft 0.1 (`docs/specs/2026-09-16-warrant-v1-spec.md`, written by Fable on Trey's direction with Trey's rulings R1 to R9 recorded in section 20.1), the research the spec cites (`docs/research/`), the agent-builder wish list and Astra's Atlas quality research (`docs/design/`), and the agent scaffolding (AGENTS.md, CLAUDE.md, beads). There is no code, no plan, and no build authorization. Origin is Forgejo `estate/warrant`; the public mirror is GitHub `treygoff24/warrant`, pushed on Trey's explicit authorization of 2026-09-16.

RESEARCH STATE. Delivered and folded into the spec: TypeSafe System One and Jev (`2026-09-16-typesafe-jev.md`), signing and attestation (`2026-09-16-signing-and-attestation.md`), Rust building blocks (`2026-09-16-rust-building-blocks.md`), polyglot integrations (`2026-09-16-polyglot-integrations.md`). In flight on Sol lanes at the time of this entry: the TypeScript analysis stack (`2026-09-16-typescript-analysis-stack.md`, which fills the `[verify: ts-stack]` markers in spec section 19) and prior art (`2026-09-16-prior-art.md`). If either file is absent, its lane did not deliver; the spec's section 25 lists both as references regardless.

RULINGS MADE BY FABLE UNDER DELEGATION (2026-09-16, recorded in spec section 20): R10, license Apache-2.0 OR MIT dual; D1 recommendation, the policy directory is visible `warrant/`; D3, judgments never block and only ever set `approval: required`. Downsides are recorded beside each. Trey can reverse any of them before the first outside contribution at no cost.

NEXT: Trey and Astra review the spec in this repository; Fable and Trey discuss it; when the three agree, `writing-plans` compiles the ratified spec into a plan and bead graph. Open decisions D1 to D12 are in spec section 20.2 with recommendations. Nothing about the build is authorized by this entry.

STANDING: commits and Forgejo pushes ungated; GitHub gated. Agents draft rulings and never sign them. No time estimates in any document. Trey dictates; read for intent.
