#!/usr/bin/env bash
set -Eeuo pipefail
cargo fmt --all -- --check
