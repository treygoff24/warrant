# type-only

Proves whole-declaration and named type-only imports keep `type_only = true`.

Positive: the two type bindings are marked and the value binding is not. Negative:
a value-only import is false. Break control: flattening those flags fails the fixture.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.
