#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generated="$(mktemp -d /var/tmp/warrant-schemas.XXXXXX)"
trap 'rm -rf "$generated"' EXIT

cd "$repo_root"
cargo run --locked --quiet -p warrant-core --bin warrant-core-schema -- "$generated"
diff -ru -- "$repo_root/schemas" "$generated"
