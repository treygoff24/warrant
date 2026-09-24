# Generated ignored: positive

Spec 5.3: "Generated files that are gitignored and therefore absent from the
snapshot are recorded as `generated-absent` with their producer". A literal
declaration's only file is gitignored and a copy is on disk after setup. Git
lists it as the one ignored untracked file, so the inventory keeps it as a
single `ignored` entry and names the declaration and its producer in
`summary.generated_absent`.
