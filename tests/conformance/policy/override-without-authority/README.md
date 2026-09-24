# Override without authority

Hard control: Spec 7.6 requires a ruling reference for explicit overrides.

- Positive: a ruling reference satisfies structural authority syntax; signature validation is a separate control.
- Negative: this policy must report `override-without-authority` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
