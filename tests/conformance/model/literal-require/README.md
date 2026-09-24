# literal-require

Proves a global CommonJS require with one literal argument is observed.

Positive: the call produces an unresolved require edge. Negative: a shadowed require
produces none. Break control: resolving or dropping the edge fails the fixture.
