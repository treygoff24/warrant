#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stages_dir="$repo_root/scripts/stages"
required_file="$stages_dir/REQUIRED"
required_stages=()
executed_stages=()

cd "$repo_root"

# Give this checkout its own cargo target directory.
#
# When a target directory is configured globally, every checkout of this
# repository shares one directory and therefore one uplifted binary path. A
# stage that runs a built binary can then execute a binary produced by a
# different checkout, which makes the gate's result depend on what other
# checkouts built rather than on the code under test. Keying the directory by
# checkout path keeps concurrent worktrees from ever sharing an uplift.
# WARRANT_GATE_TARGET_DIR overrides this for a caller that isolates builds
# itself.
if [ -n "${WARRANT_GATE_TARGET_DIR:-}" ]; then
  CARGO_TARGET_DIR="$WARRANT_GATE_TARGET_DIR"
else
  target_base="${CARGO_TARGET_DIR:-}"
  if [ -z "$target_base" ]; then
    target_base="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin).get("target_directory",""))' 2>/dev/null || true)"
  fi
  [ -n "$target_base" ] || target_base="$repo_root/target"
  checkout_key="$(printf '%s' "$repo_root" | sha256sum | cut -c1-16)"
  CARGO_TARGET_DIR="$target_base/gate-$checkout_key"
fi
export CARGO_TARGET_DIR

rustc --version
cargo --version
cargo clippy --version
cargo deny --version
python3 --version
if mutants_version="$(cargo mutants --version 2>/dev/null)"; then
  printf '%s\n' "$mutants_version"
fi
git --version
if command -v node >/dev/null 2>&1; then
  node --version
fi
printf '%s\n' "gate: target dir $(basename -- "$CARGO_TARGET_DIR")"

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
  if ! { [ -f "$stages_dir/$stage" ] && [ -x "$stages_dir/$stage" ]; }; then
    printf '%s\n' "gate: required stage is missing or not executable: $stage" >&2
    exit 1
  fi
  required_stages+=("$stage")
done <"$required_file"

export LC_ALL=C
for stage in "$stages_dir"/*; do
  [ -f "$stage" ] && [ -x "$stage" ] || continue
  entry="$(basename -- "$stage")"
  printf '%s\n' "gate: running $entry"
  "$stage" </dev/null
  executed_stages+=("$entry")
done

failed=0
if [ "${#executed_stages[@]}" -eq 0 ]; then
  printf '%s\n' "gate: no executable stages ran" >&2
  failed=1
fi
for required in "${required_stages[@]}"; do
  found=0
  for executed in "${executed_stages[@]}"; do
    [ "$required" = "$executed" ] && { found=1; break; }
  done
  if [ "$found" -eq 0 ]; then
    printf '%s\n' "gate: required stage did not run: $required" >&2
    failed=1
  fi
done
for executed in "${executed_stages[@]}"; do
  found=0
  for required in "${required_stages[@]}"; do
    [ "$executed" = "$required" ] && { found=1; break; }
  done
  if [ "$found" -eq 0 ]; then
    printf '%s\n' "gate: executed stage is absent from REQUIRED: $executed" >&2
    failed=1
  fi
done
[ "$failed" -eq 0 ] || exit 1

printf '%s\n' "gate: OK"
