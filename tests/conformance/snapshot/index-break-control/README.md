# Index snapshot: break the control

Hard control: `snapshot --index` reads staged bytes, not a later worktree edit.
The staged and worktree versions intentionally differ, and the emitted tree
must equal `git write-tree`.
