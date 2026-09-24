HC7: package imports use the captured package map.

Positive: #leaf resolves to leaf.ts and value. Negative: removing the imports entry leaves an unresolved edge rather than a guessed package. Break the control: erase index.ts; the runner rejects the missing edge.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
