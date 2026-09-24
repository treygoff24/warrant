"""Run pinned corpus members through the real snapshot and inventory CLI."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parents[3]


def redact_paths(message, corpus_dir):
    roots = [(Path(corpus_dir).resolve(), "<corpus>"), (Path(tempfile.gettempdir()).resolve(), "<tmp>")]
    for root, replacement in sorted(roots, key=lambda entry: len(str(entry[0])), reverse=True):
        pattern = re.escape(str(root)) + r"(?=/|$|[\s'\"<>:,;()\[\]])(?:/[^\s'\"<>:,;()\[\]]+)*"
        message = re.sub(pattern, replacement, message)
    return message


def run(args, cwd=ROOT, env=None, *, public=False, corpus_dir=None, index_file=None):
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
    if index_file is not None:
        environment["GIT_INDEX_FILE"] = str(index_file)
    result = subprocess.run(
        args,
        cwd=cwd,
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        message = f"{Path(args[0]).name} {args[1]} exited {result.returncode}"
        # Private members never expose child output; public errors keep bounded diagnostics.
        if public and result.stderr:
            lines = result.stderr.splitlines()[-20:]
            message += "\n" + redact_paths("\n".join(lines), corpus_dir)
        raise RuntimeError(message)
    return result.stdout.strip()


def observe(binary, member, corpus_dir, destination):
    environment = dict(os.environ, WARRANT_CORPUS_DIR=str(corpus_dir))
    public = member["public_fetch"]
    run(
        [
            str(ROOT / "scripts/corpus.sh"),
            "unpack",
            member["name"],
            str(destination),
        ],
        env=environment,
        public=public,
        corpus_dir=corpus_dir,
    )
    # Track exactly the archived source, including tracked ignored files. The
    # unpacked dependency bundle remains an analysis input, never source.
    with tarfile.open(corpus_dir / member["source_artifact"]) as archive:
        paths = [
            str(Path(entry.name).relative_to(member["name"]))
            for entry in archive
            if entry.isfile() or entry.issym()
        ]
    assert paths, "empty corpus source"
    run(["git", "init", "-q", "--object-format=sha1"], destination, public=public, corpus_dir=corpus_dir)
    for offset in range(0, len(paths), 200):
        run(["git", "add", "-f", "--", *paths[offset : offset + 200]], destination, public=public, corpus_dir=corpus_dir)
    run(
        [
            "git",
            "-c",
            "user.name=Corpus",
            "-c",
            "user.email=corpus@example.invalid",
            "commit",
            "-qm",
            "pinned corpus",
        ],
        destination,
        public=public,
        corpus_dir=corpus_dir,
    )
    with tempfile.TemporaryDirectory(prefix="warrant-corpus-cache-") as cache:
        environment["XDG_CACHE_HOME"] = cache
        snapshots = {}
        tree = "sha1:" + run(["git", "write-tree"], destination, public=public, corpus_dir=corpus_dir)
        for kind in ("index", "commit"):
            args = ["--index"] if kind == "index" else ["--commit", "HEAD"]
            document = json.loads(
                run([binary, "snapshot", *args], destination, environment, public=public, corpus_dir=corpus_dir)
            )
            assert document["tree"] == tree, "snapshot differs from pinned source tree"
            snapshots[kind] = {
                key: document[key] for key in ("schema_version", "kind", "excluded")
            }
        with tempfile.TemporaryDirectory(prefix="warrant-corpus-index-") as temporary_index:
            index_file = Path(temporary_index) / "index"
            # Git must retain tracked files even when an ignore rule now matches them.
            run(["git", "read-tree", "HEAD"], destination, public=public, corpus_dir=corpus_dir, index_file=index_file)
            run(["git", "add", "-A"], destination, public=public, corpus_dir=corpus_dir, index_file=index_file)
            worktree_tree = "sha1:" + run(
                ["git", "write-tree"],
                destination,
                public=public,
                corpus_dir=corpus_dir,
                index_file=index_file,
            )
        document = json.loads(
            run([binary, "snapshot", "--worktree"], destination, environment, public=public, corpus_dir=corpus_dir)
        )
        assert document["tree"] == worktree_tree, "worktree snapshot differs from Git staged worktree"
        manifest = destination / "warrant/warrant.yaml"
        assert not manifest.exists(), "corpus already carries Warrant configuration"
        manifest.parent.mkdir(exist_ok=True)
        manifest.write_text(
            (Path(__file__).parent / "warrant.yaml").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        inventory = json.loads(run([binary, "inventory"], destination, environment, public=public, corpus_dir=corpus_dir))
        assert inventory["total"] > 0 and not inventory["truncated"], (
            "empty or truncated inventory"
        )
        # Spec 5.5: by_class breaks down the files the snapshot holds. The listing also
        # carries each ignored path, and corpus/warrant.yaml declares no generated files,
        # so the worktree listing is exactly files plus ignored files.
        summary = inventory["summary"]
        assert sum(summary["by_class"].values()) == summary["files"], "by_class does not sum to files"
        assert inventory["total"] == summary["files"] + summary["ignored_files"], (
            "total is not files plus ignored files"
        )
        # The unowned-source count returns at W2.1 with CLI module selectors;
        # until then it only restates by_class.source for corpus/warrant.yaml.
        return {
            "snapshot": snapshots,
            "inventory": {
                "schema_version": inventory["schema_version"],
                "files": inventory["summary"]["files"],
                "by_class": inventory["summary"]["by_class"],
                "unread_count": len(inventory["summary"]["unread"]),
            },
        }


def main():
    binary = str(Path(sys.argv[1]).resolve())
    default_corpus_dir = Path(os.environ.get("TMPDIR", "/tmp")) / "warrant-corpus"
    corpus_dir = Path(os.environ.get("WARRANT_CORPUS_DIR", default_corpus_dir))
    members = json.loads((ROOT / "tests/corpus/manifest.yaml").read_text())["members"]
    default_expectations = Path(__file__).parent / "expect.json"
    expected = json.loads(default_expectations.read_text(encoding="utf-8"))
    assert {member["name"] for member in members} == set(expected), (
        "corpus expectation inventory differs"
    )
    selected = sys.argv[2:]
    assert set(selected) <= {member["name"] for member in members}, (
        "unknown selected corpus member"
    )
    failures = []
    for member in members:
        if selected and member["name"] not in selected:
            continue
        name = member["name"]
        try:
            present = [
                (corpus_dir / member[key]).is_file()
                for key in ("source_artifact", "dependency_artifact")
            ]
            if not any(present) and not member["public_fetch"]:
                print(f"corpus: {name} not run: private member unavailable", flush=True)
                continue
            if not all(present) and member["public_fetch"]:
                run(
                    [str(ROOT / "scripts/corpus.sh"), "fetch", name],
                    env=dict(os.environ, WARRANT_CORPUS_DIR=str(corpus_dir)),
                    public=True,
                    corpus_dir=corpus_dir,
                )
            with tempfile.TemporaryDirectory(prefix="warrant-corpus-") as temporary:
                actual = observe(binary, member, corpus_dir, Path(temporary))
            if actual != expected[name]:
                raise RuntimeError("semantic snapshot/inventory expectation mismatch")
            print(f"corpus: {name} ran", flush=True)
        except (AssertionError, RuntimeError) as error:
            failures.append(name)
            print(f"corpus: {name} FAILED: {error}", flush=True)
    if failures:
        raise RuntimeError(f"{len(failures)} member(s) failed")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, RuntimeError) as error:
        print(f"corpus: FAIL {error}", file=sys.stderr)
        sys.exit(1)
