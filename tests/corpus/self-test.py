"""Exercise corpus controls in an isolated, disposable corpus, without network I/O."""

import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def archive(path, files, links=()):
    with tarfile.open(path, "w:gz") as output:
        for name, data in files.items():
            info = tarfile.TarInfo(name)
            info.size = len(data)
            output.addfile(info, io.BytesIO(data))
        for name, target, kind in links:
            info = tarfile.TarInfo(name)
            info.type = kind
            info.linkname = target
            output.addfile(info)


def main():
    original, manifest_path, corpus_path = map(Path, sys.argv[1:])
    manifest = json.loads(manifest_path.read_text())
    seed = next(m for m in manifest["members"] if m["name"] == "pnpm-project-references")
    seed_archive = corpus_path / seed["source_artifact"]
    assert seed_archive.is_file(), "self-test requires the pinned pnpm source archive"
    assert digest(seed_archive) == seed["tree_sha256"], "self-test seed digest mismatch"

    with tempfile.TemporaryDirectory(prefix="warrant-corpus-self-test.", dir="/var/tmp") as tmp:
        root = Path(tmp)
        script = root / "scripts/corpus.sh"
        script.parent.mkdir()
        shutil.copy2(original, script)
        manifest_file = root / "tests/corpus/manifest.yaml"
        manifest_file.parent.mkdir(parents=True)
        corpus = root / "corpus"
        corpus.mkdir()
        scratch = root / "scratch"
        scratch.mkdir()
        env = dict(os.environ, WARRANT_CORPUS_DIR=str(corpus), TMPDIR=str(scratch))
        first = copy.deepcopy(seed)
        source = corpus / first["source_artifact"]
        shutil.copyfile(seed_archive, source)
        dependencies = corpus / first["dependency_artifact"]
        archive(dependencies, {"node_modules/example/index.js": b"module.exports = 1;\n"})
        first["dependency_bundle_sha256"] = digest(dependencies)
        baseline = {"inputs": manifest["inputs"], "members": [first]}

        def write(data):
            manifest_file.write_text(json.dumps(data))

        def run(args, expected, message):
            result = subprocess.run([str(script), *args], env=env, text=True, capture_output=True)
            output = result.stdout + result.stderr
            assert result.returncode == expected, (args, result.returncode, output)
            assert message in output, (args, output)
            return output

        def shell(function, *args):
            subprocess.run(
                ["bash", "-c", f'source "$1"; {function} "${{@:2}}"', "self-test", str(script), *map(str, args)],
                env=env, check=True, capture_output=True,
            )

        write(baseline)
        run(["verify"], 0, "verified 1/1 members")
        intact = source.read_bytes()
        flipped = bytes([intact[0] ^ 1]) + intact[1:]
        assert flipped != intact and len(flipped) == len(intact)
        source.write_bytes(flipped)
        run(["verify"], 1, "source digest mismatch")
        source.write_bytes(intact)
        run(["verify"], 0, "verified 1/1 members")

        empty = copy.deepcopy(baseline)
        empty["members"] = []
        assert not empty["members"]
        write(empty)
        run(["verify"], 1, "no corpus members")

        second = dict(first, name="second", source_artifact="second-source.tar.gz")
        second_source = corpus / second["source_artifact"]
        with tarfile.open(source) as bundle:
            lock = bundle.extractfile(f'{first["name"]}/{first["lockfile"]}').read()
        archive(second_source, {f'second/{first["lockfile"]}': lock})
        second["tree_sha256"] = digest(second_source)
        pair = dict(baseline, members=[first, second])
        assert len({m["name"] for m in pair["members"]}) == 2
        write(pair)
        run(["verify", first["name"], "second"], 0, "verified 2/2 members")
        second_bytes = second_source.read_bytes()
        second_source.write_bytes(bytes([second_bytes[0] ^ 1]) + second_bytes[1:])
        assert digest(second_source) != second["tree_sha256"]
        run(["verify", first["name"], "second"], 1, "second source digest mismatch")

        malformed = copy.deepcopy(baseline)
        member = malformed["members"][0]
        assert member["public_fetch"] is True
        member["renamed_public_fetch"] = member.pop("public_fetch")
        assert "public_fetch" not in member
        write(malformed)
        run(["fetch", "all"], 1, "public_fetch")

        wrong_lock = copy.deepcopy(baseline)
        wrong_lock["members"][0]["lockfile_sha256"] = "0" * 64
        assert hashlib.sha256(lock).hexdigest() != "0" * 64
        write(wrong_lock)
        run(["verify"], 1, "lockfile digest mismatch")

        wrong_platform = copy.deepcopy(baseline)
        wrong_platform["inputs"] = dict(baseline["inputs"], platform_observed="not-this-platform")
        write(wrong_platform)
        output = run(["fetch", first["name"]], 1, "platform mismatch")
        assert "dependency digest mismatch" not in output

        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE):
            in_tree = "../example/index.js" if kind == tarfile.SYMTYPE else "node_modules/example/index.js"
            for target, expected in (("/outside", 1), ("../../../outside", 1), (in_tree, 0)):
                archive(dependencies, {"node_modules/example/index.js": b"ok\n"},
                        [("node_modules/.bin/example", target, kind)])
                with tarfile.open(dependencies) as bundle:
                    link = bundle.getmember("node_modules/.bin/example")
                    assert link.linkname == target and link.type == kind
                links_manifest = copy.deepcopy(baseline)
                links_manifest["members"][0]["dependency_bundle_sha256"] = digest(dependencies)
                write(links_manifest)
                destination = root / f"unpack-{kind.decode()}-{expected}-{len(target)}"
                run(["unpack", first["name"], str(destination)], expected,
                    "unsafe archive link" if expected else "unpacked")

        tree = root / "tree"
        (tree / "node_modules/.bin").mkdir(parents=True)
        (tree / "node_modules/example").mkdir()
        (tree / "node_modules/example/index.js").write_text("ok\n")
        (tree / "node_modules/.bin/example").symlink_to("../example/index.js")
        assert list(tree.iterdir()) == [tree / "node_modules"]
        legacy, current = root / "legacy.tar.gz", root / "current.tar.gz"
        subprocess.run([
            "bash", "-o", "pipefail", "-c",
            'tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner '
            '--hard-dereference --format=pax --pax-option=delete=atime,delete=ctime '
            '-C "$1" -cf - node_modules | gzip -n > "$2"',
            "legacy", str(tree), str(legacy),
        ], check=True, env=env)
        shell("make_dependency_archive", tree, current)
        assert legacy.read_bytes() == current.read_bytes(), "single-root archive bytes changed"
        nested = tree / "packages/child/node_modules"
        nested.mkdir(parents=True)
        (nested / "example").symlink_to("../../../node_modules/example")
        shell("make_dependency_archive", tree, current)
        with tarfile.open(current) as bundle:
            names = bundle.getnames()
            assert "packages/child/node_modules/example" in names
            assert names == sorted(names), "archive roots are not globally sorted"

        # A local Git source and fake installer test promotion without network I/O.
        repository = root / "repository"
        repository.mkdir()
        (repository / first["lockfile"]).write_bytes(lock)
        subprocess.run(["git", "init", "-q", str(repository)], check=True)
        subprocess.run(["git", "-C", str(repository), "add", first["lockfile"]], check=True)
        subprocess.run([
            "git", "-C", str(repository), "-c", "user.name=Corpus test",
            "-c", "user.email=corpus@example.invalid", "commit", "-qm", "fixture",
        ], check=True)
        commit = subprocess.check_output(["git", "-C", str(repository), "rev-parse", "HEAD"], text=True).strip()
        fetched_source = root / "fetched-source.tar.gz"
        shell("make_source_archive", first["name"], repository, commit, fetched_source)
        installer = root / "bin/npx"
        installer.parent.mkdir()
        installer.write_text("#!/bin/sh\nmkdir -p node_modules\nprintf 'installed\\n' > node_modules/fixture\n")
        installer.chmod(0o755)
        env["PATH"] = f'{installer.parent}:{env["PATH"]}'
        fetch_manifest = copy.deepcopy(baseline)
        fetch_member = fetch_manifest["members"][0]
        fetch_member.update(source=str(repository), commit=commit, package_manager="npm@fixture",
                            tree_sha256=digest(fetched_source), dependency_bundle_sha256="0" * 64)
        assert digest(source) != fetch_member["tree_sha256"]
        previous = {p.name: p.read_bytes() for p in corpus.iterdir()}
        assert not list(scratch.iterdir())
        write(fetch_manifest)
        run(["fetch", first["name"]], 1, "dependency digest mismatch")
        assert previous == {p.name: p.read_bytes() for p in corpus.iterdir()}, "failed fetch replaced corpus bytes"
        assert not list(scratch.iterdir()), "failed fetch leaked its temporary directory"
        (repository / "node_modules").mkdir()
        (repository / "node_modules/fixture").write_text("installed\n")
        shell("make_dependency_archive", repository, current)
        fetch_member["dependency_bundle_sha256"] = digest(current)
        fetch_manifest["members"].append(dict(first, name="private", public_fetch=False))
        write(fetch_manifest)
        run(["fetch", "all"], 0, "fetched 1/1 public members")
        run(["verify", first["name"]], 0, "verified 1/1 members")
        assert not list(scratch.iterdir()), "successful fetch leaked its temporary directory"

    print("self-test: violation detected")


if __name__ == "__main__":
    main()
