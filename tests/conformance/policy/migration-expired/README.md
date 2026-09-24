# Migration expired

Hard control: Spec 7.6 reports an expired migration still present in policy; the contract stops applying at expiry and compilation continues.

- Positive: an unexpired migration applies and produces no report.
- Negative: this policy compiles (exit 0), carries one `expired-contract` report naming `module.one`, and lists no contracts, because the only contract has expired.
- Break the control: aborting compilation on the expiry fails the exit code, and keeping the expired contract or dropping the report fails the `contract_ids` and `reports` expectations.
