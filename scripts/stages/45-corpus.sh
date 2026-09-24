#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
python3 -B - <<'PY'
import unittest

suite = unittest.defaultTestLoader.discover("tests/conformance/corpus", pattern="run.py")
count = suite.countTestCases()
print(f"corpus: runner unit tests {count}", flush=True)
if count == 0:
    raise SystemExit("corpus: no runner unit tests found")
if not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful():
    raise SystemExit(1)
PY
cargo build --locked --quiet -p warrant-cli
binary="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"] + "/debug/warrant")')"
python3 tests/conformance/corpus/run.py "$binary"
