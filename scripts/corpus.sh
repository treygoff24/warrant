#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
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
  name=$1
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
  printf 'corpus: %s verified\n' "$name"
}

verify() {
  require_manifest
  status=0
  if [ "$#" -eq 0 ]; then
    names=$(member_names)
  else
    names=$1
  fi
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    verify_member "$name" || status=1
  done <<EOF
$names
EOF
  return "$status"
}

make_source_archive() {
  name=$1
  repository=$2
  commit=$3
  output=$4
  git -C "$repository" archive --format=tar --prefix="$name/" "$commit" | gzip -n > "$output"
}

make_dependency_archive() {
  repository=$1
  output=$2
  tar \
    --sort=name \
    --mtime=@0 \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    --hard-dereference \
    --format=pax \
    --pax-option=delete=atime,delete=ctime \
    -C "$repository" -cf - node_modules | gzip -n > "$output"
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

fetch_one() {
  name=$1
  source=$(member_field "$name" source)
  public=$(member_field "$name" public_fetch)
  [ "$public" = true ] || fail "$name is private and must be provisioned out of band"

  commit=$(member_field "$name" commit)
  package_manager=$(member_field "$name" package_manager)
  source_artifact=$(member_field "$name" source_artifact)
  dependency_artifact=$(member_field "$name" dependency_artifact)
  mkdir -p "$CORPUS_DIR"

  tmp=$(mktemp -d "${TMPDIR:-/tmp}/warrant-corpus.${name}.XXXXXX")
  case "$tmp" in
    "${TMPDIR:-/tmp}"/warrant-corpus."$name".*) ;;
    *) fail "refusing unexpected temporary path" ;;
  esac

  git init -q "$tmp/repository"
  git -C "$tmp/repository" remote add origin "$source"
  git -C "$tmp/repository" fetch -q --depth 1 origin "$commit"
  fetched=$(git -C "$tmp/repository" rev-parse FETCH_HEAD)
  [ "$fetched" = "$commit" ] || fail "$name fetched $fetched, expected $commit"
  git -C "$tmp/repository" checkout -q --detach "$commit"

  make_source_archive "$name" "$tmp/repository" "$commit" "$tmp/$source_artifact"
  (
    cd "$tmp/repository"
    install_dependencies "$tmp/repository" "$package_manager"
  )
  [ -d "$tmp/repository/node_modules" ] || fail "$name install produced no node_modules"
  make_dependency_archive "$tmp/repository" "$tmp/$dependency_artifact"

  mv "$tmp/$source_artifact" "$CORPUS_DIR/$source_artifact"
  mv "$tmp/$dependency_artifact" "$CORPUS_DIR/$dependency_artifact"
  verify_member "$name"
  case "$tmp" in
    "${TMPDIR:-/tmp}"/warrant-corpus."$name".*) rm -rf -- "$tmp" ;;
    *) fail "refusing unexpected temporary path during cleanup" ;;
  esac
}

fetch() {
  require_manifest
  [ "$#" -eq 1 ] || fail "usage: scripts/corpus.sh fetch <name|all>"
  if [ "$1" = all ]; then
    while IFS= read -r name; do
      [ "$(member_field "$name" public_fetch)" = true ] || continue
      fetch_one "$name"
    done < <(member_names)
  else
    fetch_one "$1"
  fi
}

check_archive_paths() {
  archive=$1
  python3 - "$archive" <<'PY'
import pathlib
import sys
import tarfile

with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive.getmembers():
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe archive path: {member.name}")
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

case ${1:-} in
  verify) shift; verify "$@" ;;
  fetch) shift; fetch "$@" ;;
  unpack) shift; unpack "$@" ;;
  *) fail "usage: scripts/corpus.sh {verify [name]|fetch <name|all>|unpack <name> <empty-directory>}" ;;
esac
