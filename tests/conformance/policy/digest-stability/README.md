# Policy digest stability

Hard control: moving contracts between files, reordering them, and reformatting YAML does not change `policy_digest`.

- Positive: compile `warrant/original/`; it produces an effective policy.
- Negative: compile `warrant/reformatted/`; its source paths and bytes differ while its policy digest equals the positive case.
- Break the control: include source paths, source bytes, or input order in the digest and the equality assertion fails.
