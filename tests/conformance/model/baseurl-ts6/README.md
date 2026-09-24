HC7: TS6 removes bare baseUrl lookup while preserving explicit paths anchors.

Positive: @leaf resolves through paths rooted at src. Negative: leaf remains module-not-found even though src/leaf.ts exists. The lang-ts version matrix also checks legacy TS5 and TS7. Break the control: restore baseUrl fallback, remove it from the paths anchor, or erase index.ts; expectations fail.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
