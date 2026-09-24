HC7 and resolution completeness: unmatched exports cannot fall back to main or vanish.

Positive control: package-conditions resolves matching import and require branches. Negative: this package exposes only browser, so its edge retains package-path-not-exported and counts as unresolved. Break the control: bypass exports using main, drop the edge, or erase index.ts; expectations fail.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
