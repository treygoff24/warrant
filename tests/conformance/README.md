# Conformance fixtures

Each case is copied into a temporary Git repository. `expect.json` describes
semantic fields, not serialized bytes; timestamps, object IDs, and JSON key
order are deliberately outside the comparison.

Every M0 control has an ordinary positive case, a non-applicable or boundary
case, and a `break-control` case. The runner also mutates the observed
unowned-source and unread-exclusion fields in memory and proves the same
expectation turns red.

## Specgate seeds

- `inventory/classification-overlap-*` ports Specgate's adversarial
  `ownership-overlap` shape: two declarations select one source path. Warrant
  separates class and ownership, so the seed now proves conflicting explicit
  classes are rejected without first-match-wins behavior.
- `inventory/colocated-test-*` ports the `test-helper-leak` shape, including a
  helper below `src/__tests__`. It proves a module selector owns the path but
  does not turn a test into source.
- `inventory/unowned-source-*` ports the `basic-project` two-module shape and
  adds an orphan source path. It proves a TypeScript file remains visible when
  no module owns it.

The registry-removal proof is assigned to W2.2, when capability evaluation
exists. The unread-file-in-contract-scope proof is assigned to W2.1, when
snapshot-dependent obligations exist. M0 still covers unread snapshot entries
without claiming the later facet transition.
