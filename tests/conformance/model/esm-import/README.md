# esm-import

Proves binding-level ESM import observation without claiming resolution.

Positive: a named import produces one unresolved import edge. Negative: an empty
source produces none. Break control: removing the edge or resolving it fails the
CLI model-row assertion.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
