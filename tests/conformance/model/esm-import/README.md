# esm-import

Proves binding-level ESM import observation without claiming resolution.

Positive: a named import produces one unresolved import edge. Negative: an empty
source produces none. Break control: removing the edge or resolving it fails the
language fixture assertion.
