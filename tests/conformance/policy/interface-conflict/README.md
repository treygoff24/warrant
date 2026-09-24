# Interface conflict

Hard control: Spec 7.6 rejects interface exports absent from the declared module interface.

- Positive: replacing the missing export with run names a declared export.
- Negative: this policy must report `interface-export-missing` without a source snapshot.
- Break the control: accepting this policy loses the required diagnostic and fails the CLI expectation.
