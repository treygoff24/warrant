# Migration missing expiry

Hard control: Spec 7.6 requires every migration to name an expiry.

- Positive: an invariant needs no expiry.
- Negative: this policy must report `migration-missing-expiry` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
