# Directory-component case collision: break the control

Control: two paths whose only case difference is a directory component fail.
`Src/one.ts` and `src/two.ts` must report `case-collision` for `Src and src`.
Whole-path-only case folding misses this pair and must fail this expectation.
The case-collision-positive and case-collision-negative fixtures retain the
successful distinct-path controls; case-collision-break-control covers filenames.
