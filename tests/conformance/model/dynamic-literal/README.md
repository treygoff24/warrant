# dynamic-literal

Proves a dynamic import with a literal target is observed without claiming resolution.

Positive: the expression produces an unresolved dynamic-import edge. Negative: an
empty source produces none. Break control: resolving or dropping it fails the fixture.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
