# unsupported

Proves TypeScript binding forms outside this analyzer are recorded explicitly.

Positive: import-equals and export-assignment each produce a reasoned unsupported row.
Negative: ordinary ESM syntax does not. Break control: silently accepting either form
fails the fixture.

The CLI check reads persisted rows with `warrant model --rows`. Replacing
`repo/index.ts` with `export const nothing = 1;` must fail this case; the
conformance suite exercises that break control on a temporary fixture copy.

Regression evidence (W1.2-FIX-2, 2026-09-24):

- The original capability-only expectations exited 0 even after replacing
  `repo/index.ts` with `export const nothing = 1;`. This demonstrated the gap.
- The new row expectations failed before the CLI supported `--rows` (exit 101;
  the command reported `invalid-invocation`).
- With row output available, the same source replacement made
  `model_conformance` exit 101: unsupported rows were empty, but the expectation
  required both `ts-import-equals` and `ts-export-assignment`. The fixture was
  restored before the closing run. `model_cases_reject_empty_sources` retains
  this negative control for all seven cases, using isolated copies.
- The planted-temporary unit test exited 101 against the original publisher:
  `a model build temporary file already exists`. The corrected publisher
  preserves both the old PID-named file and a collision at the first candidate,
  publishes a SQLite model, and cleans its own temporary directory.

The temporary reservation uses atomic directory creation and a bounded sequence
of process-specific names: the existing ModelBuilder requires that its database
path does not yet exist, so it is created inside the reserved directory.
