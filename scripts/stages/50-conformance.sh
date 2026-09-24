#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo test --locked -p warrant-cli --test conformance -- inventory
cargo test --locked -p warrant-cli --test conformance -- snapshot
