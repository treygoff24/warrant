"""Run acceptance demos through the public CLI seam."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]


def run(args, cwd=ROOT, env=None):
    environment = dict(
        os.environ if env is None else env,
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL=os.devnull,
    )
    for key in (
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ):
        environment.pop(key, None)
    result = subprocess.run(
        args,
        cwd=cwd,
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, f"{Path(args[0]).name} exited {result.returncode}"
    return result.stdout


def inventory():
    run(["cargo", "build", "--locked", "--quiet", "-p", "warrant-cli"])
    metadata = json.loads(
        run(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    )
    binary = str(Path(metadata["target_directory"]) / "debug/warrant")
    fixture = ROOT / "tests/conformance/inventory/colocated-test-positive"
    with tempfile.TemporaryDirectory(prefix="warrant-demo-3-") as temporary:
        repository = Path(temporary) / "repo"
        shutil.copytree(fixture / "repo", repository)
        shutil.copytree(fixture / "warrant", repository / "warrant")
        (repository / "outside.ts").write_text(
            "export const outside = 1;\n", encoding="utf-8"
        )
        run(["git", "init", "-q"], repository)
        run(["git", "add", "."], repository)
        run(
            [
                "git",
                "-c",
                "user.name=Acceptance",
                "-c",
                "user.email=acceptance@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
            repository,
        )
        document = json.loads(
            run(
                [binary, "inventory"],
                repository,
                dict(
                    os.environ,
                    XDG_CACHE_HOME=str(Path(temporary) / "cache"),
                ),
            )
        )
        entries = {entry["path"]: entry for entry in document["entries"]}
        assert entries["src/app.test.ts"]["class"] == "test", (
            "colocated test lost its class"
        )
        assert "outside.ts" in document["summary"]["unowned_source"], (
            "unowned first-party source was omitted"
        )
        # F39 leaves module-owned CLI assertions to W2.1; W0.5 binds them in-crate.


if __name__ == "__main__":
    number = sys.argv[1]
    try:
        {"3": inventory}[number]()
    except (AssertionError, KeyError) as error:
        print(f"REFUSE {number} {error}")
        sys.exit(1)
    print(f"ACCEPT {number}")
