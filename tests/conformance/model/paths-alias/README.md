HC7: resolution uses the owning tsconfig paths table.

Positive: @lib/leaf resolves to lib/leaf.ts and its value export. Negative: the lang-ts resolution test retains a missing sibling import with module-not-found. Break the control: removing the alias or erasing index.ts must fail the expected target assertion.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
