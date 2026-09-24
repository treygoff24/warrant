#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

if cargo test --quiet --locked -p warrant-cli --test conformance -- snapshot_index_acceptance \
  >/dev/null 2>&1; then
  printf '%s\n' 'ACCEPT 2 index snapshot equals the staged Git tree'
else
  printf '%s\n' 'REFUSE 2 index snapshot did not equal the staged Git tree'
  exit 1
fi
