HC7: export conditions follow importer format and load syntax.

Positive: index.ts selects the import branch, other.cts selects require, and its dynamic import selects import. Negative: no-export-condition exercises a package with no matching branch. Break the control: force every edge to require or erase index.ts; semantic target assertions fail.

These cases do not prove tsc parity; resolution remains unqualified pending S6.
