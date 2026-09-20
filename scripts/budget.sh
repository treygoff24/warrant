#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
failed=0

for crate in core snapshot inventory model lang-ts lang-rust evidence authority judgment census render cli; do
  case "$crate" in
    core) limit=8000 ;;
    lang-ts) limit=6000 ;;
    authority) limit=3000 ;;
    cli) limit=4000 ;;
    *) limit=2500 ;;
  esac

  lines=0
  while IFS= read -r -d '' source; do
    count="$(wc -l <"$source")"
    lines=$((lines + count))
  done < <(/usr/bin/find "$repo_root/crates/$crate/src" -type f -name '*.rs' -print0)

  printf '%s\n' "budget: $crate $lines/$limit"
  if [ "$lines" -gt "$limit" ]; then
    failed=1
  fi
done

exit "$failed"
