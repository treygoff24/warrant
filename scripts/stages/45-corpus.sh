#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$repo_root/tests/corpus/manifest.yaml"
expectations="${WARRANT_CORPUS_EXPECTATIONS:-$repo_root/tests/conformance/corpus/expect.json}"
corpus_dir="${WARRANT_CORPUS_DIR:-${TMPDIR:-/tmp}/warrant-corpus}"
fixture_manifest="$repo_root/tests/conformance/corpus/warrant.yaml"
scratch=""

cleanup() {
  if [ -n "$scratch" ] && [ -d "$scratch" ]; then
    rm -rf -- "$scratch"
  fi
}
trap cleanup EXIT

fail() {
  printf 'corpus: %s\n' "$*" >&2
  exit 1
}

[ -f "$manifest" ] || fail "manifest is missing"
[ -f "$expectations" ] || fail "semantic expectations are missing"
[ -f "$fixture_manifest" ] || fail "analysis manifest is missing"

cd "$repo_root"
cargo build --quiet --locked -p warrant-cli --bin warrant
target_dir="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
warrant="$target_dir/debug/warrant"
[ -x "$warrant" ] || fail "warrant binary was not built"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/warrant-corpus-stage.XXXXXX")"

while IFS=$'\t' read -r name public source_artifact expected_digest; do
  [ -n "$name" ] || continue
  archive="$corpus_dir/$source_artifact"
  if [ ! -f "$archive" ]; then
    if [ "$public" = false ]; then
      printf 'corpus: %s not run: private member unavailable\n' "$name"
      continue
    fi
    fail "$name source artifact is unavailable"
  fi
  actual_digest="$(sha256sum "$archive" | cut -d' ' -f1)"
  [ "$actual_digest" = "$expected_digest" ] || fail "$name source digest mismatch"

  repository="$scratch/$name"
  mkdir -p "$repository"
  tar --no-same-owner --no-same-permissions -xzf "$archive" --strip-components=1 -C "$repository"
  mkdir -p "$repository/warrant"
  cp "$fixture_manifest" "$repository/warrant/warrant.yaml"
  git -C "$repository" init -q
  git -C "$repository" add -f . # FOOTGUN-OK: isolated freshly-extracted corpus repository
  git -C "$repository" \
    -c user.name='Warrant Corpus' \
    -c user.email=warrant@example.invalid \
    commit -qm fixture

  snapshot_json="$scratch/$name.snapshot.json"
  inventory_json="$scratch/$name.inventory.json"
  set +e
  (cd "$repository" && XDG_CACHE_HOME="$scratch/cache-$name" "$warrant" snapshot --worktree) \
    >"$snapshot_json" 2>&1
  snapshot_exit=$?
  (cd "$repository" && XDG_CACHE_HOME="$scratch/cache-$name" "$warrant" inventory) \
    >"$inventory_json" 2>&1
  inventory_exit=$?
  set -e

  python3 - "$expectations" "$name" "$snapshot_exit" "$snapshot_json" \
    "$inventory_exit" "$inventory_json" <<'PY'
import json
import sys

expect_path, name, snapshot_exit, snapshot_path, inventory_exit, inventory_path = sys.argv[1:]
expected = json.load(open(expect_path, encoding="utf-8"))["members"].get(name)
if expected is None:
    raise SystemExit(f"corpus: {name} has no semantic expectation")

def load(path):
    try:
        return json.load(open(path, encoding="utf-8"))
    except Exception as error:
        raise SystemExit(f"corpus: {name} emitted invalid JSON: {error}")

snapshot = load(snapshot_path)
want_snapshot = expected["snapshot"]
if int(snapshot_exit) != want_snapshot["exit_code"]:
    raise SystemExit(f"corpus: {name} snapshot exit {snapshot_exit}, expected {want_snapshot['exit_code']}")
for field in ("kind", "object_format"):
    if snapshot.get(field) != want_snapshot[field]:
        raise SystemExit(f"corpus: {name} snapshot {field} differs")

inventory = load(inventory_path)
want_inventory = expected["inventory"]
if int(inventory_exit) != want_inventory["exit_code"]:
    raise SystemExit(f"corpus: {name} inventory exit {inventory_exit}, expected {want_inventory['exit_code']}")
if int(inventory_exit) == 0:
    summary = inventory["summary"]
    if summary["files"] != want_inventory["files"]:
        raise SystemExit(f"corpus: {name} inventory file count differs")
    if summary["by_class"] != want_inventory["by_class"]:
        raise SystemExit(f"corpus: {name} inventory class count differs")
else:
    if inventory.get("code") != want_inventory["error_code"]:
        raise SystemExit(f"corpus: {name} inventory error code differs")
    if want_inventory["reason_contains"] not in inventory.get("reason", ""):
        raise SystemExit(f"corpus: {name} inventory error reason differs")
PY
  printf 'corpus: %s ran\n' "$name"
done < <(python3 - "$manifest" <<'PY'
import json
import sys

for member in json.load(open(sys.argv[1], encoding="utf-8"))["members"]:
    print("\t".join((
        member["name"],
        "true" if member["public_fetch"] else "false",
        member["source_artifact"],
        member["tree_sha256"],
    )))
PY
)
