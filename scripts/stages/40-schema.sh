#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generated="$(mktemp -d /var/tmp/warrant-schemas.XXXXXX)"
cleanup() {
  if [[ -n "$generated" && "$generated" == /var/tmp/warrant-schemas.?????? && -d "$generated" ]]; then
    rm -rf -- "$generated"
  fi
}
trap cleanup EXIT

cd "$repo_root"
cargo run --locked --quiet -p warrant-core --bin warrant-core-schema -- "$generated"
# The stage asks whether the checked-in generated schemas are current. Hand-written
# documentation beside them (schemas/model.md, spec 6.2) is not generated and is
# excluded; every JSON file on either side is still compared.
diff -ru --exclude='*.md' -- "$repo_root/schemas" "$generated"
