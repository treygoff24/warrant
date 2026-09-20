#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

check_dependencies() {
  local root="$1"
  local metadata
  metadata="$(cargo metadata --format-version=1 --no-deps --manifest-path "$root/Cargo.toml")"

  METADATA="$metadata" python - <<'PY'
import json
import os
import sys

metadata = json.loads(os.environ["METADATA"])
workspace = {package["id"]: package for package in metadata["packages"]}
by_name = {package["name"] for package in workspace.values()}

core_only = {
    "warrant-snapshot",
    "warrant-inventory",
    "warrant-model",
    "warrant-evidence",
    "warrant-authority",
    "warrant-judgment",
    "warrant-census",
    "warrant-render",
}
language = {"warrant-lang-ts", "warrant-lang-rust"}

violations = []
for package in workspace.values():
    source = package["name"]
    for dependency in package["dependencies"]:
        target = dependency["name"]
        if target not in by_name:
            continue
        allowed = (
            source == "warrant-cli"
            or (source in core_only and target == "warrant-core")
            or (source in language and target in {"warrant-core", "warrant-model"})
        )
        if not allowed:
            violations.append(f"deps: forbidden workspace edge: {source} -> {target}")

if violations:
    print("\n".join(sorted(violations)), file=sys.stderr)
    raise SystemExit(1)
PY
}

self_test() {
  local tmp_root
  tmp_root="$(mktemp -d "${TMPDIR:-/var/tmp}/warrant-deps.XXXXXX")"
  cleanup() {
    case "$tmp_root" in
      "${TMPDIR:-/var/tmp}"/warrant-deps.*) rm -rf -- "$tmp_root" ;;
      *) printf '%s\n' "deps self-test: refused unsafe cleanup path" >&2 ;;
    esac
  }
  trap cleanup RETURN

  cp "$repo_root/Cargo.toml" "$repo_root/Cargo.lock" "$tmp_root/"
  cp -R "$repo_root/crates" "$tmp_root/crates"
  cat >>"$tmp_root/crates/core/Cargo.toml" <<'EOF'

[target.'cfg(warrant_deps_self_test)'.dependencies]
warrant-snapshot = { version = "0.1.0", path = "../snapshot" }
EOF

  local output
  if output="$(check_dependencies "$tmp_root" 2>&1)"; then
    printf '%s\n' "self-test: detector accepted a forbidden edge" >&2
    return 1
  fi
  if ! command grep -q 'warrant-core -> warrant-snapshot' <<<"$output"; then
    printf '%s\n' "self-test: detector failed for the wrong reason" >&2
    printf '%s\n' "$output" >&2
    return 1
  fi
  printf '%s\n' "self-test: violation detected"
}

case "${1:-}" in
  "") check_dependencies "$repo_root" ;;
  --self-test) self_test ;;
  *)
    printf '%s\n' "usage: scripts/deps.sh [--self-test]" >&2
    exit 2
    ;;
esac
