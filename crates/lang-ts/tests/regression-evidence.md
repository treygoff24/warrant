# W1.2-FIX-1 regression evidence

Observed on 2026-09-24 with `cargo test --locked -p warrant-lang-ts` and
individual test-name filters. Every red run below exited 101. Mutations were
applied individually and removed before the closing suite.

| Control | Red observation before correction |
| --- | --- |
| Unique symbol IDs | Original allocator: `duplicate id 8589934595` in the two-file alias/default-export test. |
| Local binding references | Original import-only gate: local call reference count was 0, expected 1. |
| Invalid source encoding | Original UTF-8 boundary: `Invalid("captured source ... is not UTF-8")` aborted the unit. |
| Observed support coverage | Adding `fake-support` to capabilities failed equality against the four support names derived from actual edges. |
| Type-only re-export | Forcing named re-export `type_only` false failed with `missing reexport edge`. |
| Unit discovery | Suppressing non-root inventory units failed the expected root/nested unit list. |
| Unsupported treatments | Individually disabling parse-error, semantic-error, require-nonliteral, ts-import-type, ts-namespace-export, and source-type emission failed the corresponding construct assertion. |

Golden rows are compared without order dependence. Spans must be contained in
fixture syntax; the local binding-reference test pins identifier text because
that span is itself the reference contract. Capability coverage ignores fixture
`supports` labels and uses analyzed edges.
