# literal-require

Proves a global CommonJS require with one literal argument is observed.

Positive: the call produces an unresolved require edge. Negative: a shadowed require
produces none. Break control: resolving or dropping the edge fails the fixture.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
