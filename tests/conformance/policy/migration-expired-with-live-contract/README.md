# Migration expired beside a live contract

Hard control: Spec 7.6 says an expired migration stops applying at expiry; it must not stop the rest of the policy from applying.

- Positive: the unexpired invariant `module.live` stays in the effective policy.
- Negative: the expired migration `module.expired` is absent from `contracts` and appears in one `expired-contract` report.
- Break the control: aborting compilation on the expiry fails the exit code and hides `module.live`; keeping `module.expired` fails `contract_ids`; dropping the report fails `reports`.
