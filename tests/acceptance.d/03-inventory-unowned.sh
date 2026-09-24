#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

if cargo test --quiet --locked -p warrant-cli --test conformance -- inventory_unowned_acceptance \
  >/dev/null 2>&1; then
  printf '%s\n' 'ACCEPT 3 unowned first-party source remains in the denominator'
else
  printf '%s\n' 'REFUSE 3 unowned first-party source disappeared from the denominator'
  exit 1
fi
