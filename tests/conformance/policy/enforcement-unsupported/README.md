# Unsupported enforcement

Hard control: a contract cannot claim binding-reference evidence while selecting pattern-only enforcement.

- Positive: change enforcement to `static`; the data contract compiles.
- Negative: pattern enforcement produces `enforcement-unsupported` and names `binding-references`.
- Break the control: trust the editable `requires_capabilities` field or the enforcement label and the contract compiles.
