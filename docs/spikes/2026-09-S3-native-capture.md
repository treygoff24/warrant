# S3: native capture with `gix`

Status: not met

## Decision

Keep native capture off. The pinned `gix` surface exposes APIs for
constructing index trees and filtered worktree blobs, but this run could
not execute the parity matrix: `warrant-snapshot` does not yet declare the
`native-capture` feature. Its manifest is outside this lane's write boundary;
the plan requires the coordinator to re-sequence manifest changes. This is a
blocked experiment, not a measured disagreement between gix and Git. Source
inspection is not parity evidence. Native implementation and tests remain
undelivered.

## Question and frozen source

Can `gix` 0.87.1 reproduce `git write-tree` tree IDs for index and worktree
captures containing untracked, modified, ignored, symlinked, and executable
files? The inspected repository source was frozen at
`3d7405d8fa21d38cf5ca298ac3822fd4b057b39a`.

## Measurements

No parity case ran. `cargo test --locked -p warrant-snapshot --features
native-capture` exited 101 because the package does not contain that feature.
`WARRANT_CORPUS_DIR` was unset; that records only this run's environment and
does not establish that the private corpus is unavailable.

The baseline `cargo test --locked -p warrant-snapshot --all-features` exited
0 with 21 passing unit tests and zero doc tests. With no feature declared,
that command exercises the native stub returning `None`, not native hashing.
The report-marker check initially exited 2 because this report did not exist.
The explicit feature invocation is the red prerequisite reproduction; no
native behavioral red/green test has run.

Observed tools: Git 2.47.3, rustc 1.98.1, Cargo 1.98.1, Linux x86_64.

None of the frozen members in `tests/corpus/manifest.yaml` ran: `atlas`,
`nextjs-app`, `pnpm-project-references`, `commonjs-library`, `vite-library`,
and `path-aliases-exports`. Neither tree ID was measured for any member.
Green fixture parity on this devbox would not establish parity on other
filesystems.

| Case | Native tree ID | `git write-tree` tree ID | Result |
| --- | --- | --- | --- |
| index baseline | not run | not run | blocked by missing feature |
| untracked file | not run | not run | blocked by missing feature |
| modified tracked file | not run | not run | blocked by missing feature |
| ignored file | not run | not run | blocked by missing feature |
| symlink | not run | not run | blocked by missing feature |
| executable file | not run | not run | blocked by missing feature |

The exit criterion requires identical IDs in every row. It is **not met**.

## Pinned API findings

- Index trees have a small native route: `Repository::open_index()`, reject any
  entry whose `Entry::stage()` is not `Stage::Unconflicted`, start from
  `Repository::edit_tree(ObjectId::empty_tree(repo.object_hash()))`, add each
  path with `Editor::upsert(path, mode.kind(), entry.id)`, where `mode` is the
  non-`None` result of `Entry::mode.to_tree_entry_mode()`, then call
  `Editor::write()`. The editor writes changed trees to
  the object database, validates path components, and verifies referenced
  non-tree objects exist. [Pinned `edit_tree` docs](https://docs.rs/gix/0.87.1/gix/struct.Repository.html#method.edit_tree),
  [`gix-0.87.1/src/object/tree/editor.rs`](https://docs.rs/gix/0.87.1/src/gix/object/tree/editor.rs.html#228-327),
  and [`gix-index-0.55.0` entry access](https://docs.rs/gix-index/0.55.0/gix_index/struct.State.html#method.entries).
- Worktree bytes need a separate attribute/filter path. Create an attribute
  stack, pass it to `gix::filter::Pipeline::new`, then call
  `worktree_file_to_object(&mut self, rela_path: &BStr, index:
  &gix_index::State)`. It applies to-Git filters for files, writes symlink
  targets without filtering, preserves the executable mode from metadata, and
  returns a gitlink for an openable nested repository. It explicitly performs
  no submodule cross-check. [Pinned pipeline docs](https://docs.rs/gix/0.87.1/gix/filter/struct.Pipeline.html#method.worktree_file_to_object)
  and [`gix-0.87.1/src/filter.rs`](https://docs.rs/gix/0.87.1/src/gix/filter.rs.html#227-275).
- Ignore decisions are separate from blob creation. `Worktree::excludes(
  overrides: Option<gix_ignore::Search>) -> Result<AttributeStack<'_>, Error>`
  loads standard user and repository excludes; callers query it per path with
  `AttributeStack::at_path`. [Pinned excludes docs](https://docs.rs/gix/0.87.1/gix/struct.Worktree.html#method.excludes)
  and [`gix-0.87.1/src/attribute_stack.rs`](https://docs.rs/gix/0.87.1/src/gix/attribute_stack.rs.html#34-63).
- The workspace pin enables both `sha1` and `sha256`, and `Repository::object_hash()`
  drives empty-tree, blob, and tree hashing. That is compile-time capability,
  not empirical SHA-256 parity. [`gix` 0.87.1 features](https://docs.rs/crate/gix/0.87.1/features).

## Boundaries to test after the prerequisite lands

- Sparse indexes can contain `Mode::DIR`; a worktree-only walk must retain its
  indexed subtree instead of treating an absent sparse path as deleted.
- `worktree_file_to_object` can treat any openable nested repository as a
  gitlink. The matrix must distinguish declared submodules from embedded repos
  and compare moved or dirty submodule HEADs with Git.
- Clean/process filters may execute configured helpers. The parity case must
  cover required-filter failure and verify the same trusted configuration and
  attributes are used as Git.
- The existing `native::capture` result is only `Option<String>`. In the
  worktree composition, `Some(id)` therefore supplies empty `carried` and
  `oversize` sets. Enabling native worktree capture would silently lose the
  temporary-index path's carried gitlink/skip-worktree protection and oversize
  annotations unless that interface is widened or native worktree capture is
  left unsupported.

## Smallest next step

The coordinator must declare `native-capture` in `crates/snapshot/Cargo.toml`
or reassign that file, keeping it out of the default feature set. Implement
and test native capture, then measure the cases against both SHA-1 and SHA-256
repositories. Enable it by default only after every recorded tree ID matches.
