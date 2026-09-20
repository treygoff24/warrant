#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stages_dir="$repo_root/scripts/stages"
required_file="$stages_dir/REQUIRED"

cd "$repo_root"

rustc --version
cargo --version
cargo clippy --version
cargo deny --version
if command -v cargo-mutants >/dev/null 2>&1; then
  cargo mutants --version
fi
git --version
if command -v node >/dev/null 2>&1; then
  node --version
fi

if [ ! -f "$required_file" ]; then
  printf '%s\n' "gate: required stage inventory is missing: $required_file" >&2
  exit 1
fi

while IFS= read -r stage || [ -n "$stage" ]; do
  stage="${stage%%#*}"
  stage="$(printf '%s' "$stage" | tr -d '[:space:]')"
  [ -n "$stage" ] || continue
  case "$stage" in
    */*)
      printf '%s\n' "gate: invalid REQUIRED entry: $stage" >&2
      exit 1
      ;;
  esac
  if [ ! -x "$stages_dir/$stage" ]; then
    printf '%s\n' "gate: required stage is missing or not executable: $stage" >&2
    exit 1
  fi
done <"$required_file"

while IFS= read -r entry; do
  stage="$stages_dir/$entry"
  [ -f "$stage" ] && [ -x "$stage" ] || continue
  printf '%s\n' "gate: running $entry"
  "$stage"
done < <(command ls -1 "$stages_dir" 2>/dev/null | LC_ALL=C sort)

printf '%s\n' "gate: OK"
