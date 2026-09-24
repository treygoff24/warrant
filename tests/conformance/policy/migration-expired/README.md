# Migration expired

Hard control: Spec 7.6 reports an expired migration still present in policy.

- Positive: an invariant without an expiry is not an expired migration.
- Negative: this policy must report `expired-contract` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
