# Generated ignored: break the control

Hard control: a glob declaration whose only matches are gitignored files is
`generated-absent` with its producer (spec 5.3), even though copies are on
disk. Deciding presence against the exclusion listing instead of captured
paths, as the inventory once did, makes this case red.
