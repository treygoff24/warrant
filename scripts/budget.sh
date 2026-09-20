#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
failed=0
source_list="$(mktemp "${TMPDIR:-/var/tmp}/warrant-budget.XXXXXX")"
member_list=""
trap 'rm -f -- "$source_list" ${member_list:+"$member_list"}' EXIT
member_list="$(mktemp "${TMPDIR:-/var/tmp}/warrant-budget-members.XXXXXX")"

if ! cargo metadata --format-version=1 --no-deps --manifest-path "$repo_root/Cargo.toml" >"$source_list"; then
  printf '%s\n' "budget: cargo metadata failed or is unavailable" >&2
  exit 1
fi
if ! python - "$source_list" >"$member_list" <<'PY'
import json
import os
import sys

with open(sys.argv[1]) as source:
    metadata = json.load(source)
members = metadata["workspace_members"]
if not isinstance(members, list) or not members:
    raise ValueError("expected nonempty workspace_members")
packages = {package["id"]: package for package in metadata["packages"]}
for member in members:
    manifest = packages[member]["manifest_path"]
    if (not isinstance(manifest, str) or not os.path.isabs(manifest)
            or os.path.basename(manifest) != "Cargo.toml" or "\0" in manifest):
        raise ValueError("invalid workspace member manifest_path")
    sys.stdout.buffer.write(os.fsencode(os.path.dirname(manifest)) + b"\0")
PY
then
  printf '%s\n' "budget: cannot parse cargo metadata workspace members" >&2
  exit 1
fi

export LC_ALL=C
while IFS= read -r -d '' crate_dir; do
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
done <"$member_list"

exit "$failed"
