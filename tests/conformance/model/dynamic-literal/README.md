# dynamic-literal

Proves a dynamic import with a literal target is observed without claiming resolution.

Positive: the expression produces an unresolved dynamic-import edge. Negative: an
empty source produces none. Break control: resolving or dropping it fails the fixture.
