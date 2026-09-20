# S3: native capture with `gix`

Status: met (six-case fixture matrix)

## Decision

All six measured fixtures produce identical native and Git tree IDs. Keep
native capture off by default: qualification over the frozen project corpus
is still unmeasured, and the production `native::capture` still returns
`None`. These measurements exercise a test-local implementation, not a
shipping capture path. The broader S3 exit criterion in spec section 17.7
has not been established; no measured divergence was found in these fixtures.

`crates/snapshot/Cargo.toml` now declares `native-capture = []` with no default
feature entry. The previous missing-feature prerequisite is resolved. Feature
wiring and default selection concern shipping only: the unconditional `gix`
dependency lets this integration target measure parity without any feature.
No manifest change is required for the six-case measurement.

## Question and source

Can pinned `gix` 0.87.1 reproduce `git write-tree` IDs for an index baseline
and worktrees with untracked, modified, ignored, symlinked, and executable
files? This measurement was made on 2026-09-20 from repository base
`6df9d1cf3a71c401574d8e22a81693263b402c65`, with the integration test added in
this change at `crates/snapshot/tests/native.rs`.

Observed tools: Git 2.47.3, rustc 1.98.1, Cargo 1.98.1, Linux x86_64. Both
the workspace and the fixture temporary directory use ZFS. Repositories are
explicitly SHA-1, with `core.fileMode=true` and `core.autocrlf=false`.
Global/system Git configuration is excluded; attributes and excludes files
outside the fixture are disabled. Native repositories use isolated open
options. The Unix-only target does not establish parity on other filesystems
or operating systems.

## Fixture corpus and method

Each test creates a fresh tempfile repository with staged `src/file.txt`
containing `baseline\n` and `.gitattributes` containing `*.txt text eol=lf\n`.
The index baseline rebuilds those index entries. The other cases change only
the worktree: add `src/new.txt` with CRLF bytes, modify `src/file.txt` with
CRLF bytes, add `.gitignore` plus an ignored file and directory, add a symlink
to `src/file.txt`, or add an executable shell script. The CRLF cases exercise
attribute-driven blob conversion as well as tree assembly.

The native index route reads `open_index()` entries. The native worktree route
enumerates fixture files without following symlinks, asks
`Worktree::excludes(None)` and `at_entry` for ignore decisions, and hashes each
included file with `filter_pipeline(None)` and `worktree_file_to_object`.
Both routes assemble trees through `edit_tree` from the empty tree, using
`Editor::upsert` and `write`. No Git subprocess computes the native result.

The reference uses a separate temporary `GIT_INDEX_FILE` outside the worktree:
`read-tree --empty`, `add -A`, then `write-tree`. This follows the temporary
index shape of `portable()` for these simple fixtures; it does not exercise
that function's carried entries or oversize handling. Native capture runs
first, so Git cannot prepopulate the worktree blobs under measurement. Each
case asserts equal tree IDs and unchanged original index bytes. Additional
assertions check the staged baseline, untracked precondition, modified tree,
ignored paths, symlink mode and target, and executable mode.

## Measurements

All rows executed. IDs below are the emitted values from
`cargo test --locked -p warrant-snapshot --test native -- --nocapture`.

| Case | Native tree ID | `git write-tree` tree ID | Result |
| --- | --- | --- | --- |
| index baseline | e133cdebf6ade6edb94c77645144487701bd5533 | e133cdebf6ade6edb94c77645144487701bd5533 | measured match |
| untracked file | d34eaf67f45181fcc6aee8e6dff96035fb1dd933 | d34eaf67f45181fcc6aee8e6dff96035fb1dd933 | measured match |
| modified tracked file | 0f108b169aa784f6c20b9e6b74b438e639b62f5a | 0f108b169aa784f6c20b9e6b74b438e639b62f5a | measured match |
| ignored file | 675204872cb6e2dd6d03565f3d0f4382be1b34ad | 675204872cb6e2dd6d03565f3d0f4382be1b34ad | measured match |
| symlink | d0e67e91813179bfbc7740920a6fedd6657f4123 | d0e67e91813179bfbc7740920a6fedd6657f4123 | measured match |
| executable file | 6a3c4588750bdd7efccae95d739d002425af07db | 6a3c4588750bdd7efccae95d739d002425af07db | measured match |

The six-case parity criterion is met: 6 passed, 0 failed. Before adding the
target, its test command exited 101 because the native test target was absent.
With the assertions installed and the native helper deliberately returning
the empty tree `4b825dc642cb6eb9a060e54bf8d69288fbee4904`, the same command
exited 101: all six tree-ID assertions failed. Replacing that empty-tree
helper with the gix implementation made all six pass. This behavioral red
result was an intentional control, not evidence of a gix divergence.

## Unmeasured scope and shipping follow-up

The frozen repositories in `tests/corpus/manifest.yaml` are outside this
fixture run: `atlas`, `nextjs-app`, `pnpm-project-references`,
`commonjs-library`, `vite-library`, and `path-aliases-exports`. Their native
and reference IDs remain unmeasured. No availability claim is made about
their artifacts.

The fixture walker is deliberately test-local. It does not qualify tracked
ignored files, sparse/skip-worktree entries, submodules or embedded
repositories, external clean/process filters and their failures, SHA-256,
or `core.fileMode=false`. These require measurement before adopting it as a
production capture algorithm.

Production worktree capture also needs to preserve the portable path's
`carried` and `oversize` sets. Its current `Option<String>` native interface
cannot return them. Changing that interface and its caller exceeds this
lane's owned files. The shipping path remains the portable implementation,
including when the feature is explicitly enabled.

## API sources

The implementation was checked against the pinned dependency source shipped
with gix 0.87.1:

- [Filter pipeline construction](https://docs.rs/gix/0.87.1/src/gix/repository/filter.rs.html)
  and [worktree blob conversion](https://docs.rs/gix/0.87.1/src/gix/filter.rs.html).
- [Worktree excludes](https://docs.rs/gix/0.87.1/gix/struct.Worktree.html#method.excludes)
  and [per-entry ignore lookup](https://docs.rs/gix/0.87.1/src/gix/attribute_stack.rs.html).
- [Tree editor](https://docs.rs/gix/0.87.1/src/gix/object/tree/editor.rs.html).

These APIs supply the mechanism; the six measured rows supply the parity
evidence.
