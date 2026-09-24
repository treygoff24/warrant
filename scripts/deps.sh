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
env_toolchains = re.findall(r"(?m)^      RUSTUP_TOOLCHAIN:[ \t]*['\"]?([^'\"\s#]+)", job["body"]) if job else []
if len(env_toolchains) != 1:
    found = ", ".join(env_toolchains) if env_toolchains else "<missing>"
    violations.append(f"deps: CI job msrv RUSTUP_TOOLCHAIN {found} differs from workspace rust-version {workspace_msrv}")
else:
    env_msrv = env_toolchains[0]
    if not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", env_msrv) or tuple(env_msrv.split(".")[:2]) != tuple(str(workspace_msrv).split(".")[:2]):
        violations.append(f"deps: CI job msrv RUSTUP_TOOLCHAIN {env_msrv} differs from workspace rust-version {workspace_msrv}")

manifest_node = json.loads((root / "tests/corpus/manifest.yaml").read_text())["inputs"]["node_observed"]
expected_node = manifest_node.removeprefix("v")
jobs = re.findall(r"(?ms)^  ([\w-]+):[^\n]*\n(.*?)(?=^  [\w-]+:|\Z)", ci)
gate_jobs = 0
for job_name, body in jobs:
    steps = re.findall(r"(?ms)^      - (.*?)(?=^      - |\Z)", body)
    gate_indices = [index for index, step in enumerate(steps) if "scripts/gate.sh" in step]
    if not gate_indices:
        continue
    gate_jobs += 1
    for index in gate_indices:
        if not re.search(r"(?m)^(?:run|        run):[ \t]*scripts/gate\.sh[ \t]*$", steps[index]):
            violations.append(f"deps: CI job {job_name} has unsupported gate invocation")
    versions = []
    setup_indices = []
    for index, step in enumerate(steps):
        if not re.search(r"(?m)^(?:uses|        uses):[ \t]*actions/setup-node@v[0-9]+[ \t]*$", step):
            continue
        setup_indices.append(index)
        versions.extend(re.findall(r"(?m)^          node-version:[ \t]*['\"]?([^'\"\s#]+)", step) or ["<missing>"])
    if any(not any(setup < gate for setup in setup_indices) for gate in gate_indices):
        violations.append(f"deps: CI job {job_name} setup-node must precede gate step")
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

  local ci_copy case_name expected
  ci_copy="$tmp_root/.github/workflows/ci.yml"
  for case_name in block_gate late_node wrong_msrv_env; do
    cp "$repo_root/.github/workflows/ci.yml" "$ci_copy"
    python3 - "$ci_copy" "$case_name" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
case = sys.argv[2]
text = path.read_text()
gate = "      - name: Run gate\n        run: scripts/gate.sh\n"
setup = "      - uses: actions/setup-node@v7\n        with:\n          node-version: 26.9.0\n"
if case == "block_gate":
    assert text.count(gate) == 2
    text = text.replace(gate, "      - name: Run gate\n        run: |\n          scripts/gate.sh\n", 1)
elif case == "late_node":
    assert text.count(setup + gate) == 2
    text = text.replace(setup + gate, gate + setup, 1)
elif case == "wrong_msrv_env":
    selector = "      RUSTUP_TOOLCHAIN: 1.96.0\n"
    assert text.count(selector) == 1
    text = text.replace(selector, "      RUSTUP_TOOLCHAIN: 1.95.0\n", 1)
else:
    raise SystemExit(f"unknown deps self-test case: {case}")
path.write_text(text)
PY
    if output="$(check_dependencies "$tmp_root" 2>&1)"; then
      printf 'self-test: detector accepted %s\n' "$case_name" >&2
      return 1
    fi
    case "$case_name" in
      block_gate) expected='deps: CI job stable has unsupported gate invocation' ;;
      late_node) expected='deps: CI job stable setup-node must precede gate step' ;;
      wrong_msrv_env) expected='deps: CI job msrv RUSTUP_TOOLCHAIN 1.95.0 differs from workspace rust-version 1.96' ;;
    esac
    if ! command grep -Fxq "$expected" <<<"$output"; then
      printf 'self-test: %s failed for the wrong reason\n%s\n' "$case_name" "$output" >&2
      return 1
    fi
    printf 'self-test: %s\n' "$expected"
  done
  cp "$repo_root/.github/workflows/ci.yml" "$ci_copy"
}

case "${1:-}" in
  "") check_dependencies "$repo_root" ;;
  --self-test) self_test ;;
  *)
    printf '%s\n' "usage: scripts/deps.sh [--self-test]" >&2
    exit 2
    ;;
esac
