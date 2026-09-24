# dynamic-nonliteral

Proves a non-literal dynamic import is never guessed.

Positive: the expression is unresolved with `dynamic-nonliteral` and has a matching
unsupported row. Negative: a literal import is not unsupported. Break control: any
target or resolved claim fails the fixture.
