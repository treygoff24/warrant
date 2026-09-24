#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
MANIFEST="$ROOT/tests/corpus/manifest.yaml"
CORPUS_DIR=${WARRANT_CORPUS_DIR:-${TMPDIR:-/tmp}/warrant-corpus}

fail() {
  printf 'corpus: %s\n' "$*" >&2
  exit 1
}

require_manifest() {
  [ -f "$MANIFEST" ] || fail "manifest not found: tests/corpus/manifest.yaml"
}

member_names() {
  python3 - "$MANIFEST" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)
for member in manifest["members"]:
    print(member["name"])
PY
}

member_field() {
  name=$1
  field=$2
  python3 - "$MANIFEST" "$name" "$field" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)
for member in manifest["members"]:
    if member["name"] == sys.argv[2]:
        value = member[sys.argv[3]]
        if isinstance(value, bool):
            print("true" if value else "false")
        elif value is not None:
            print(value)
        break
else:
    raise SystemExit(f"unknown corpus member: {sys.argv[2]}")
PY
}

sha256() {
  sha256sum "$1" | cut -d' ' -f1
}

verify_member() {
  local name=$1 source_artifact source_digest dependency_artifact dependency_digest
  local actual lockfile lockfile_digest
  source_artifact=$(member_field "$name" source_artifact)
  source_digest=$(member_field "$name" tree_sha256)
  dependency_artifact=$(member_field "$name" dependency_artifact)
  dependency_digest=$(member_field "$name" dependency_bundle_sha256)

  [ -f "$CORPUS_DIR/$source_artifact" ] || {
    printf 'corpus: %s missing %s\n' "$name" "$source_artifact" >&2
    return 1
  }
  [ -f "$CORPUS_DIR/$dependency_artifact" ] || {
    printf 'corpus: %s missing %s\n' "$name" "$dependency_artifact" >&2
    return 1
  }
  actual=$(sha256 "$CORPUS_DIR/$source_artifact")
  [ "$actual" = "$source_digest" ] || {
    printf 'corpus: %s source digest mismatch\n' "$name" >&2
    return 1
  }
  actual=$(sha256 "$CORPUS_DIR/$dependency_artifact")
  [ "$actual" = "$dependency_digest" ] || {
    printf 'corpus: %s dependency digest mismatch\n' "$name" >&2
    return 1
  }
  lockfile=$(member_field "$name" lockfile)
  lockfile_digest=$(member_field "$name" lockfile_sha256)
  actual=$(tar -xOzf "$CORPUS_DIR/$source_artifact" "$name/$lockfile" | sha256sum | cut -d' ' -f1)
  [ "$actual" = "$lockfile_digest" ] || {
    printf 'corpus: %s lockfile digest mismatch\n' "$name" >&2
    return 1
  }
  printf 'corpus: %s verified\n' "$name"
}

verify() {
  require_manifest
  local list name count
  local -a names
  if [ "$#" -eq 0 ]; then
    list=$(member_names)
    [ -n "$list" ] || fail "no corpus members"
    mapfile -t names <<< "$list"
  else
    names=("$@")
  fi
  count=${#names[@]}
  [ "$count" -gt 0 ] || fail "no corpus members"
  # With no arguments this is the manifest's own count, not a truncation check.
  # A truncated manifest that still lists at least one member can pass.
  for name in "${names[@]}"; do
    verify_member "$name"
  done
  printf 'corpus: verified %s/%s members\n' "$count" "$count" >&2
}

make_source_archive() {
  name=$1
  repository=$2
  commit=$3
  output=$4
  git -C "$repository" archive --format=tar --prefix="$name/" "$commit" | gzip -n > "$output"
}

make_dependency_archive() {
  local repository=$1 output=$2
  # Prune each matched root: tar visits its contents once, in name order.
  (cd "$repository" && find . -type d -name node_modules -print0 -prune) |
    LC_ALL=C sort -z | sed -z 's|^\./||' |
  tar \
    --sort=name \
    --mtime=@0 \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    --hard-dereference \
    --format=pax \
    --pax-option=delete=atime,delete=ctime \
    -C "$repository" --null -T - -cf - | gzip -n > "$output"
}

install_dependencies() {
  repository=$1
  package_manager=$2
  case "$package_manager" in
    npm@*)
      env -u NPM_TOKEN -u NODE_AUTH_TOKEN -u YARN_NPM_AUTH_TOKEN \
        NPM_CONFIG_USERCONFIG=/dev/null \
        npx --yes "$package_manager" ci --ignore-scripts --no-audit --no-fund \
        --registry=https://registry.npmjs.org
      ;;
    pnpm@*)
      env -u NPM_TOKEN -u NODE_AUTH_TOKEN -u YARN_NPM_AUTH_TOKEN \
        NPM_CONFIG_USERCONFIG=/dev/null \
        npx --yes "$package_manager" install --frozen-lockfile --ignore-scripts \
        --store-dir=.pnpm-store --config.prefer-symlinked-executables=true \
        --registry=https://registry.npmjs.org
      python3 - "$repository/node_modules/.modules.yaml" <<'PY'
import json
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
if text.lstrip().startswith("{"):
    data = json.loads(text)
    if "storeDir" not in data:
        raise SystemExit(f"expected pnpm storeDir in {path}")
    data["storeDir"] = ".warrant-pnpm-store-not-bundled"
    if "prunedAt" in data:
        data["prunedAt"] = "Thu, 01 Jan 1970 00:00:00 GMT"
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
else:
    text, count = re.subn(r"(?m)^storeDir: .*$", "storeDir: .warrant-pnpm-store-not-bundled", text)
    if count != 1:
        raise SystemExit(f"expected one pnpm storeDir in {path}, found {count}")
    text = re.sub(r"(?m)^prunedAt: .*$", "prunedAt: Thu, 01 Jan 1970 00:00:00 GMT", text)
    path.write_text(text, encoding="utf-8")
PY
      if [ -f "$repository/node_modules/.pnpm-workspace-state-v1.json" ]; then
        python3 - "$repository/node_modules/.pnpm-workspace-state-v1.json" "$repository" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
root = pathlib.Path(sys.argv[2]).resolve()
data = json.loads(path.read_text(encoding="utf-8"))
data["lastValidatedTimestamp"] = 0
projects = {}
for project, value in data.get("projects", {}).items():
    relative = pathlib.Path(project).resolve().relative_to(root).as_posix()
    projects[relative or "."] = value
data["projects"] = projects
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
      fi
      ;;
    *) fail "unsupported package manager for $repository: $package_manager" ;;
  esac
}

fetch_one() (
  # Recorded dependency digests were taken under umask 002.
  # Pin it here so the caller's umask cannot decide archive modes.
  umask 002
  name=$1
  source=$(member_field "$name" source)
  public=$(member_field "$name" public_fetch)
  [ "$public" = true ] || fail "$name is private and must be provisioned out of band"

  commit=$(member_field "$name" commit)
  package_manager=$(member_field "$name" package_manager)
  source_artifact=$(member_field "$name" source_artifact)
  dependency_artifact=$(member_field "$name" dependency_artifact)
  expected_platform=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["inputs"]["platform_observed"])' "$MANIFEST")
  [ "$(uname -s -m)" = "$expected_platform" ] || fail "$name platform mismatch: expected $expected_platform"
  mkdir -p "$CORPUS_DIR"

  tmp=$(mktemp -d "${TMPDIR:-/var/tmp}/warrant-corpus.${name}.XXXXXX")
  case "$tmp" in
    "${TMPDIR:-/var/tmp}"/warrant-corpus."$name".*) ;;
    *) fail "refusing unexpected temporary path" ;;
  esac
  trap 'rm -rf -- "$tmp"' EXIT

  printf 'fetch: %s clone\n' "$name" >&2
  git init -q "$tmp/repository"
  git -C "$tmp/repository" remote add origin "$source"
  git -C "$tmp/repository" fetch -q --depth 1 origin "$commit"
  fetched=$(git -C "$tmp/repository" rev-parse FETCH_HEAD)
  [ "$fetched" = "$commit" ] || fail "$name fetched $fetched, expected $commit"
  git -C "$tmp/repository" checkout -q --detach "$commit"

  printf 'fetch: %s install\n' "$name" >&2
  (
    cd "$tmp/repository"
    install_dependencies "$tmp/repository" "$package_manager"
  )
  [ -d "$tmp/repository/node_modules" ] || fail "$name install produced no node_modules"
  printf 'fetch: %s archive\n' "$name" >&2
  make_source_archive "$name" "$tmp/repository" "$commit" "$tmp/$source_artifact"
  make_dependency_archive "$tmp/repository" "$tmp/$dependency_artifact"

  # Both archives and the lockfile must match before replacing either artifact.
  printf 'fetch: %s verify\n' "$name" >&2
  CORPUS_DIR=$tmp verify_member "$name"
  mv "$tmp/$source_artifact" "$CORPUS_DIR/$source_artifact"
  mv "$tmp/$dependency_artifact" "$CORPUS_DIR/$dependency_artifact"
)

fetch() {
  require_manifest
  local names name public fetched=0 expected=0
  [ "$#" -eq 1 ] || fail "usage: scripts/corpus.sh fetch <name|all>"
  if [ "$1" = all ]; then
    names=$(member_names)
    [ -n "$names" ] || fail "no corpus members"
    while IFS= read -r name; do
      public=$(member_field "$name" public_fetch)
      case "$public" in
        false) continue ;;
        true) expected=$((expected + 1)) ;;
        *) fail "$name invalid public_fetch" ;;
      esac
      fetch_one "$name"
      fetched=$((fetched + 1))
    done <<< "$names"
    [ "$expected" -gt 0 ] || fail "no public corpus members"
    [ "$fetched" -eq "$expected" ] || fail "public corpus member skipped"
    printf 'corpus: fetched %s/%s public members\n' "$fetched" "$expected"
  else
    fetch_one "$1"
  fi
}

check_archive_paths() {
  archive=$1
  python3 - "$archive" <<'PY'
import pathlib
import posixpath
import sys
import tarfile

with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive.getmembers():
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe archive path: {member.name}")
        if member.issym() or member.islnk():
            target = pathlib.PurePosixPath(member.linkname)
            resolved = posixpath.normpath(str(path.parent / target))
            if target.is_absolute() or resolved == ".." or resolved.startswith("../"):
                raise SystemExit(f"unsafe archive link: {member.name}")
PY
}

unpack() {
  require_manifest
  [ "$#" -eq 2 ] || fail "usage: scripts/corpus.sh unpack <name> <empty-directory>"
  name=$1
  destination=$2
  verify_member "$name" >/dev/null
  mkdir -p "$destination"
  [ -z "$(find "$destination" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    fail "unpack destination is not empty: $destination"
  }

  source_artifact=$(member_field "$name" source_artifact)
  dependency_artifact=$(member_field "$name" dependency_artifact)
  check_archive_paths "$CORPUS_DIR/$source_artifact"
  check_archive_paths "$CORPUS_DIR/$dependency_artifact"
  tar --no-same-owner --no-same-permissions -xzf "$CORPUS_DIR/$source_artifact" \
    --strip-components=1 -C "$destination"
  tar --no-same-owner --no-same-permissions -xzf "$CORPUS_DIR/$dependency_artifact" \
    -C "$destination"
  printf 'corpus: %s unpacked\n' "$name"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
case ${1:-} in
  --self-test) python3 "$ROOT/tests/corpus/self-test.py" "$ROOT/scripts/corpus.sh" "$MANIFEST" "$CORPUS_DIR" ;;
  verify) shift; verify "$@" ;;
  fetch) shift; fetch "$@" ;;
  unpack) shift; unpack "$@" ;;
  *) fail "usage: scripts/corpus.sh {verify [name...]|fetch <name|all>|unpack <name> <empty-directory>|--self-test}" ;;
esac
fi
