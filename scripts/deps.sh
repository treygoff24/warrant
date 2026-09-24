#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

check_dependencies() {
  local root="$1"
  local metadata
  metadata="$(cargo metadata --locked --format-version=1 --no-deps --manifest-path "$root/Cargo.toml")"

  METADATA="$metadata" ROOT="$root" python3 - <<'PY'
import json
import os
from pathlib import Path
import re
import sys
import tomllib

metadata = json.loads(os.environ["METADATA"])
root = Path(os.environ["ROOT"])
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
workspace_msrv = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"].get("rust-version")
ci = (root / ".github/workflows/ci.yml").read_text()
job = re.search(r"(?ms)^  msrv:\s*\n(?P<body>.*?)(?=^  [\w-]+:|\Z)", ci)
toolchains = re.findall(r"(?m)^          toolchain:\s*['\"]?(\d+\.\d+(?:\.\d+)?)['\"]?\s*$", job["body"]) if job else []
if len(toolchains) != 1:
    violations.append("deps: expected one numeric toolchain in CI msrv job")
else:
    ci_msrv = toolchains[0]
    if not isinstance(workspace_msrv, str) or not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", workspace_msrv):
        violations.append(f"deps: invalid workspace rust-version: {workspace_msrv!r}")
    elif tuple(workspace_msrv.split(".")[:2]) != tuple(ci_msrv.split(".")[:2]):
        violations.append(f"deps: workspace rust-version {workspace_msrv} differs from CI MSRV {ci_msrv}")

manifest_node = json.loads((root / "tests/corpus/manifest.yaml").read_text())["inputs"]["node_observed"]
expected_node = manifest_node.removeprefix("v")
jobs = re.findall(r"(?ms)^  ([\w-]+):[^\n]*\n(.*?)(?=^  [\w-]+:|\Z)", ci)
gate_jobs = 0
for job_name, body in jobs:
    steps = re.findall(r"(?ms)^      - (.*?)(?=^      - |\Z)", body)
    if not any(re.search(r"(?m)^(?:run|        run):[ \t]*scripts/gate\.sh[ \t]*$", step) for step in steps):
        continue
    gate_jobs += 1
    versions = []
    for step in steps:
        if not re.search(r"(?m)^(?:uses|        uses):[ \t]*actions/setup-node@v[0-9]+[ \t]*$", step):
            continue
        versions.extend(re.findall(r"(?m)^          node-version:[ \t]*['\"]?([^'\"\s#]+)", step) or ["<missing>"])
    if len(versions) != 1 or versions[0].removeprefix("v") != expected_node:
        found = ", ".join(versions) if versions else "<missing>"
        violations.append(f"deps: CI job {job_name} setup-node node-version {found} differs from corpus node_observed {manifest_node}")
if gate_jobs == 0:
    violations.append("deps: no CI job runs scripts/gate.sh")

for package in workspace.values():
    source = package["name"]
    manifest = tomllib.loads(Path(package["manifest_path"]).read_text())
    declaration = manifest["package"].get("rust-version")
    if declaration != {"workspace": True}:
        violations.append(f"deps: {source} must inherit workspace rust-version; found {declaration!r}")
    if package["rust_version"] != workspace_msrv:
        violations.append(f"deps: {source} rust-version {package['rust_version']!r} differs from workspace {workspace_msrv!r}")
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
  mkdir -p "$tmp_root/.github/workflows"
  cp "$repo_root/.github/workflows/ci.yml" "$tmp_root/.github/workflows/ci.yml"
  mkdir -p "$tmp_root/tests/corpus"
  cp "$repo_root/tests/corpus/manifest.yaml" "$tmp_root/tests/corpus/manifest.yaml"
  cat >>"$tmp_root/crates/snapshot/Cargo.toml" <<'EOF'
warrant-inventory = { version = "0.1.0", path = "../inventory" }
EOF
  cat >>"$tmp_root/crates/lang-ts/Cargo.toml" <<'EOF'
warrant-authority = { version = "0.1.0", path = "../authority" }
EOF

  local output
  if output="$(check_dependencies "$tmp_root" 2>&1)"; then
    printf '%s\n' "self-test: detector accepted a forbidden edge" >&2
    return 1
  fi
  if ! command grep -Fxq 'deps: forbidden workspace edge: warrant-snapshot -> warrant-inventory' <<<"$output" \
    || ! command grep -Fxq 'deps: forbidden workspace edge: warrant-lang-ts -> warrant-authority' <<<"$output"; then
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
