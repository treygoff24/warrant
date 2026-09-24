HC7: project references select the owning configuration, including non-default config filenames.

Positive: app/main.ts resolves @lib under app/config.json. Negative: the solution has no alias to use as a fallback if the reference is removed. Break the control: remove the reference or erase index.ts; the expected edges fail.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
