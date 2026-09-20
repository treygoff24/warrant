#!/usr/bin/env bash
set -Eeuo pipefail
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
