# unsupported

Proves TypeScript binding forms outside this analyzer are recorded explicitly.

Positive: import-equals and export-assignment each produce a reasoned unsupported row.
Negative: ordinary ESM syntax does not. Break control: silently accepting either form
fails the fixture.
