# Structural conflict

Hard control: overlapping module ownership is a configuration defect that compiles without a snapshot and is never resolved by file order.

- Positive: either module contract compiles alone.
- Negative: both contracts produce `module-files-conflict`.
- Break the control: apply first-match-wins semantics and the expected error disappears.
