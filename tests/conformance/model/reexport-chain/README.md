# reexport-chain

Proves that every link in an ESM re-export chain is observed and remains unresolved.

Positive: both source files produce a re-export edge. Negative: an empty source
produces none. Break control: dropping either edge fails the language fixture assertion.
