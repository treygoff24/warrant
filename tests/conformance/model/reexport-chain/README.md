# reexport-chain

Proves that every link in an ESM re-export chain is observed and remains unresolved.

Positive: both source files produce a re-export edge. Negative: an empty source
produces none. Break control: dropping either edge fails the CLI model-row assertion.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
