#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# Plan section 3 item 2: this binds the index half while the worktree differs.
# W0.3's crate test binds snapshot-unstable mid-capture rewrites; no M0 fixture.
output="$(mktemp)"
trap 'rm -f "$output"' EXIT
if cargo test --quiet --locked -p warrant-cli --test conformance -- --exact snapshot_index_acceptance \
  >"$output" 2>&1 && grep -q '^test result: ok. 1 passed;' "$output"; then
  printf '%s\n' 'ACCEPT 2 index snapshot equals the staged Git tree'
else
  cat "$output" >&2
  printf '%s\n' 'REFUSE 2 index snapshot did not equal the staged Git tree'
  exit 1
fi
