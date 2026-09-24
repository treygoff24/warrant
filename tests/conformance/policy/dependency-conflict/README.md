# Dependency conflict

Hard control: Spec 7.6 forbids allowing and denying the same edge without an explicit override.

- Positive: either dependency rule alone is structurally valid.
- Negative: this policy must report `dependency-conflict` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
