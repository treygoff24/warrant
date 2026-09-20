#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
failed=0
source_list="$(mktemp "${TMPDIR:-/var/tmp}/warrant-budget.XXXXXX")"
trap 'rm -f -- "$source_list"' EXIT

export LC_ALL=C
for crate_dir in "$repo_root"/crates/*; do
  [ -d "$crate_dir" ] && [ -f "$crate_dir/Cargo.toml" ] || continue
  crate="$(basename -- "$crate_dir")"
  case "$crate" in
    core) limit=8000 ;;
    lang-ts) limit=6000 ;;
    authority) limit=3000 ;;
    cli) limit=4000 ;;
    snapshot|inventory|model|lang-rust|evidence|judgment|census|render) limit=2500 ;;
    *)
      printf '%s\n' "budget: $crate has no configured limit" >&2
      failed=1
      continue
      ;;
  esac

  src_dir="$crate_dir/src"
  if [ ! -d "$src_dir" ]; then
    printf '%s\n' "budget: $crate source directory is missing: $src_dir" >&2
    failed=1
    continue
  fi
  if ! /usr/bin/find "$src_dir" -type f -name '*.rs' -print0 >"$source_list"; then
    printf '%s\n' "budget: $crate source traversal failed" >&2
    failed=1
    continue
  fi
  if [ ! -s "$source_list" ]; then
    printf '%s\n' "budget: $crate has no Rust source files" >&2
    failed=1
    continue
  fi

  lines=0
  while IFS= read -r -d '' source; do
    count="$(wc -l <"$source")"
    lines=$((lines + count))
  done <"$source_list"

  printf '%s\n' "budget: $crate $lines/$limit"
  if [ "$lines" -gt "$limit" ]; then
    failed=1
  fi
done

exit "$failed"
