# dynamic-nonliteral

Proves a non-literal dynamic import is never guessed.

Positive: the expression is unresolved with `dynamic-nonliteral` and has a matching
unsupported row. Negative: a literal import is not unsupported. Break control: any
target or resolved claim fails the fixture.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
