# Case collision: break the control

Hard control: two paths that differ only by case fail on every platform, so a
snapshot cannot resolve differently on a case-insensitive machine.

`src/Case.ts` is not checked in: the runner writes it and stages its blob beside the
checked-in `src/case.ts` (`setup.case_variant_files`), so no clone of this repository
carries a case-colliding pair.
