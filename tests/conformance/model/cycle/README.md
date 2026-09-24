HC7: resolved bindings populate the module graph and observed interfaces.

Positive: the captured module policy assigns index.ts and leaf.ts to different modules; both graph directions, the two cycle members, and both observed exports are asserted. Negative: the conformance runner erases index.ts and rejects the missing cycle and edges. Break the control: skip resolution, module-policy loading, or cycle materialization; the same expectations fail.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
