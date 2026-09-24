# State conflict

Hard control: Spec 7.6 rejects different writer sets for the same store.

- Positive: either writer rule alone is structurally valid.
- Negative: this policy must report `state-writers-conflict` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
