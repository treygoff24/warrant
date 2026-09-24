#!/usr/bin/env bash
# Warrant acceptance runner. Written by the build coordinator before launch; no
# lane owns this file (plan Interfaces, "Gate, stages, acceptance").
#
# Plain form:   tests/acceptance.sh
#   Runs scripts/gate.sh, then every executable under tests/acceptance.d/ in
#   lexical order. Each item prints "ACCEPT <n>" or "REFUSE <n> <reason>".
#   tests/acceptance.d/REQUIRED lists the item numbers that must exist and be
#   executable at this point of the build. A required item that is missing or
#   not executable, any item that prints REFUSE, and any item that exits
#   nonzero all fail this script.
#
# Final form:   tests/acceptance.sh --final M0|M1|M2
#   Everything above, plus the milestone's full item inventory (M0: 02-03,
#   M1: 02-05, M2: 01-10), the corpus stage reporting that the Atlas member
#   ran, and the schema stage reporting zero stub documents.
#
# Stage marker contract (plan Interfaces; coordinator ruling F2, 2026-09-20),
# consumed by --final and produced by the stage scripts:
#   scripts/stages/45-corpus.sh  prints one line per corpus member, either
#       "corpus: <name> ran"
#     or
#       "corpus: <name> not run: private member unavailable"
#   scripts/stages/40-schema.sh  prints one line per pre-declared document,
#       "schema: <name> implemented"   or   "schema: <name> stub"
#     in a stable order, and may follow them with a summary line
#       "schema: stub-documents <count>"
#   A bare count cannot answer the plan's condition, which is zero stubs among
#   the documents IN SCOPE at that milestone: W0.2 pre-declares all 23 and M0
#   delivers five, so a correct M0 build has 18 stubs. --final therefore checks
#   the per-milestone in-scope list below, and fails closed when a marker it
#   needs is absent, so an unmarked stage is a red result, never a silent pass.
#
# Gate output goes to .verify/acceptance-gate.log (gitignored) and is not
# echoed on success: this script's stdout is committed as
# docs/acceptance/demo-output.md at final close, and this repository is public.
# Everything this script does print is scanned for home paths and internal
# hostnames before it exits.

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

gate="scripts/gate.sh"
items_dir="tests/acceptance.d"
required_file="$items_dir/REQUIRED"
log_dir=".verify"
gate_log="$log_dir/acceptance-gate.log"

final_milestone=""
case "${1:-}" in
  "") ;;
  --final)
    final_milestone="${2:-}"
    if [ -z "$final_milestone" ]; then
      printf '%s\n' "acceptance: --final requires a milestone: M0, M1, or M2" >&2
      exit 2
    fi
    case "$final_milestone" in
      M0|M1|M2) ;;
      *)
        printf '%s\n' "acceptance: unknown milestone '$final_milestone'; expected M0, M1, or M2" >&2
        exit 2
        ;;
    esac
    ;;
  *)
    printf '%s\n' "acceptance: unknown argument '${1}'; usage: tests/acceptance.sh [--final M0|M1|M2]" >&2
    exit 2
    ;;
esac

# Everything printed to stdout is buffered here so it can be privacy-scanned
# before it reaches the caller that commits it.
out_file="$(mktemp)"
trap 'rm -f "$out_file"' EXIT

emit() { printf '%s\n' "$*" >>"$out_file"; }

failures=0
fail() {
  emit "FAIL $*"
  failures=$((failures + 1))
}

finish() {
  local rc=0
  if [ "$failures" -ne 0 ]; then
    emit "acceptance: FAILED with $failures failure(s)"
    rc=1
  else
    emit "acceptance: OK"
  fi
  # Privacy scan over our own stdout before anyone commits it.
  # stdout is captured and committed as docs/acceptance/demo-output.md, so a
  # leak is withheld from stdout entirely and reported on stderr instead.
  if command grep -nE '/home/|/Users/|\.ts\.net|100\.[0-9]+\.[0-9]+\.[0-9]+' "$out_file" >/dev/null 2>&1; then
    printf '%s\n' "acceptance: REFUSED to emit output containing a home path or internal host; this repository is public" >&2
    printf '%s\n' "offending lines (withheld from stdout):" >&2
    command grep -nE '/home/|/Users/|\.ts\.net|100\.[0-9]+\.[0-9]+\.[0-9]+' "$out_file" >&2 || true
    exit 3
  fi
  cat "$out_file"
  exit "$rc"
}

emit "warrant acceptance"
if [ -n "$final_milestone" ]; then
  emit "mode: final $final_milestone"
else
  emit "mode: plain"
fi

# ---- gate -------------------------------------------------------------------
if [ ! -x "$gate" ]; then
  fail "gate: $gate is missing or not executable"
  finish
fi

mkdir -p "$log_dir"
gate_rc=0
"$gate" >"$gate_log" 2>&1 || gate_rc=$?
if [ "$gate_rc" -ne 0 ]; then
  fail "gate: $gate exited $gate_rc"
  emit "--- last 40 lines of $gate_log ---"
  tail -n 40 "$gate_log" >>"$out_file" || true
  emit "--- end gate log ---"
  finish
fi
emit "gate: pass"

# ---- required inventory -----------------------------------------------------
if [ ! -d "$items_dir" ]; then
  fail "items: $items_dir does not exist"
  finish
fi
if [ ! -f "$required_file" ]; then
  fail "items: $required_file does not exist"
  finish
fi

required=()
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%%#*}"
  line="$(printf '%s' "$line" | tr -d '[:space:]')"
  [ -n "$line" ] || continue
  required+=("$line")
done <"$required_file"

if [ -n "$final_milestone" ]; then
  case "$final_milestone" in
    M0) milestone_items=(02 03) ;;
    M1) milestone_items=(02 03 04 05) ;;
    M2) milestone_items=(01 02 03 04 05 06 07 08 09 10) ;;
  esac
  for want in "${milestone_items[@]}"; do
    found=0
    for have in ${required[@]+"${required[@]}"}; do
      [ "$have" = "$want" ] && { found=1; break; }
    done
    if [ "$found" -eq 0 ]; then
      fail "final: milestone $final_milestone requires item $want, which $required_file does not list"
    fi
  done
fi

# Map item number -> file, over the executables actually present.
item_files=()
item_numbers=()
while IFS= read -r f; do
  [ -x "$f" ] || continue
  base="$(basename "$f")"
  case "$base" in
    REQUIRED|README.md|*.md) continue ;;
  esac
  num="$(printf '%s' "$base" | sed -n 's/^\([0-9][0-9]*\).*/\1/p')"
  if [ -z "$num" ]; then
    fail "items: $f is executable but its name does not start with an item number"
    continue
  fi
  item_files+=("$f")
  item_numbers+=("$num")
done < <(command ls -1 "$items_dir" 2>/dev/null | LC_ALL=C sort | sed "s|^|$items_dir/|")

for want in ${required[@]+"${required[@]}"}; do
  found=0
  for have in ${item_numbers[@]+"${item_numbers[@]}"}; do
    [ "$have" = "$want" ] && { found=1; break; }
  done
  if [ "$found" -eq 0 ]; then
    fail "items: required item $want is missing or not executable in $items_dir"
  fi
done

# ---- run the items ----------------------------------------------------------
count="${#item_files[@]}"
if [ "$count" -eq 0 ]; then
  emit "items: none present"
else
  idx=0
  while [ "$idx" -lt "$count" ]; do
    f="${item_files[$idx]}"
    num="${item_numbers[$idx]}"
    idx=$((idx + 1))
    # File names are zero-padded (03-foo.sh); the demo token the plan's verify
    # rows expect is not (ACCEPT 3). Accept either spelling.
    plain="$(printf '%s' "$num" | sed 's/^0*//')"
    [ -n "$plain" ] || plain=0
    pat="^(ACCEPT|REFUSE) ($num|$plain)([^0-9]|$)"
    item_out="$(mktemp)"
    item_rc=0
    "$f" >"$item_out" 2>&1 || item_rc=$?
    cat "$item_out" >>"$out_file"
    if command grep -qE "^REFUSE ($num|$plain)([^0-9]|$)" "$item_out"; then
      fail "item $num: printed REFUSE"
    elif [ "$item_rc" -ne 0 ]; then
      fail "item $num: exited $item_rc"
    elif ! command grep -qE "$pat" "$item_out"; then
      fail "item $num: printed neither ACCEPT $plain nor REFUSE $plain"
    fi
    rm -f "$item_out"
  done
fi

# ---- final-only stage assertions -------------------------------------------
if [ -n "$final_milestone" ]; then
  if command grep -q '^corpus: atlas not run' "$gate_log"; then
    fail "final: the corpus stage reports the Atlas member did not run"
  elif ! command grep -q '^corpus: atlas ran' "$gate_log"; then
    fail "final: the corpus stage printed no 'corpus: atlas ran' marker"
  fi

  # A document is in scope at a milestone when its owning task is at or before
  # it. The owning task id travels with each name so this list stays checkable
  # against the plan's owned_files rather than being trusted.
  m0_docs="snapshot:W0.2 inventory:W0.2 capabilities:W0.2 manifest:W0.2 error:W0.2 commands:W0.7"
  m1_docs="instruments:W1.4 policy:W1.7 effective-policy:W1.7 census:W1.10 context:W1.11 propose:W1.11 map:W1.12 hook:W1.13"
  m2_docs="obligations:W2.1 finding:W2.3 evidence:W2.4 test-receipt:W2.4 verdict:W2.5 receipt:W2.5 profile:W2.6 ruling:W2.7 diff:W2.8 verify:W2.9"
  case "$final_milestone" in
    M0) in_scope="$m0_docs" ;;
    M1) in_scope="$m0_docs $m1_docs" ;;
    M2) in_scope="$m0_docs $m1_docs $m2_docs" ;;
    *)  in_scope="" ;;
  esac

  scoped=0
  for entry in $in_scope; do
    doc="${entry%%:*}"
    owner="${entry##*:}"
    scoped=$((scoped + 1))
    if command grep -qE "^schema: warrant\.$doc stub\$" "$gate_log"; then
      fail "final: warrant.$doc is in scope at $final_milestone ($owner) and the schema stage reports it a stub"
    elif ! command grep -qE "^schema: warrant\.$doc implemented\$" "$gate_log"; then
      fail "final: the schema stage printed no marker for warrant.$doc, in scope at $final_milestone ($owner)"
    fi
  done
  # A check over an empty set passes and proves nothing; assert the precondition.
  if [ "$scoped" -eq 0 ]; then
    fail "final: no in-scope document list for milestone $final_milestone"
  else
    emit "final: $scoped in-scope document(s) checked at $final_milestone"
  fi
fi

finish
