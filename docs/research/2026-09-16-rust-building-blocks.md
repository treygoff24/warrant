# Rust building blocks for Warrant

Status: research report, 2026-09-16. Author: Fable (research lane), for Warrant. Versions verified live on 2026-09-16.

## Scope and method

Warrant needs content-addressed snapshots of a repository, an inventory, a program model in SQLite, contracts evaluated into a four-fact verdict, a machine-verifiable receipt, and human-signed rulings. This report surveys the Rust crates and system tools each of those needs, with versions read from the crates.io API on 2026-09-16, licenses and MSRV from the same records or from the crate manifests, and API claims checked against the current source on GitHub. Where a number could not be verified it is marked "unverified".

Every version below was read from https://crates.io/api/v1/crates/<name> on 2026-09-16 unless another source is named. Repository health numbers (stars, open issues and pull requests combined, last push) come from the GitHub REST API on the same day. The current stable Rust release is 1.98.1, published 2026-09-03 (https://github.com/rust-lang/rust/releases).

The predecessor's manifest (specgate 0.3.2, edition 2024, rust-version 1.85) depends on petgraph 0.7, globset 0.4, sha2 0.10, miette 7, thiserror 2, schemars 0.8, chrono 0.4, walkdir 2, serde_yml 0.0.12, clap 4, and optional rayon. Two of those need to change on day one: serde_yml (archived and under a RustSec advisory, see section 9) and schemars 0.8 (the 1.x line is current, see section 6).

## 1. Git access

Warrant needs to read trees and blobs by hash, hash the working tree and the index as trees, detect renames between trees, build an integrated candidate from two lanes without touching the worktree, list worktrees, and read signed commits.

### gix (gitoxide)

Version 0.87.1, released 2026-08-24, MIT OR Apache-2.0, MSRV 1.85, published by Sebastian Thiel (crates.io login Byron), as of 2026-09-16 (https://crates.io/api/v1/crates/gix). Repository https://github.com/GitoxideLabs/gitoxide: 11,956 stars, last push 2026-09-16, and only 9 open issues and pull requests combined, which is unusually low for the size of the project and probably reflects triage practice (unverified). One principal author is the bus-factor consideration.

Fit. gix covers every read we need in process, and the merge piece exists too. From the current source (https://github.com/GitoxideLabs/gitoxide/tree/main/gix/src/repository, as of 2026-09-16):

- Objects: `find_object`, `find_tree`, `find_blob`, `find_commit`, `write_blob`, and `edit_tree`, a tree editor (object.rs).
- Index: `open_index`, `index_or_load_from_head`, `index_from_tree(&oid)` (index.rs). I found no index-to-tree writer in 0.87.1; `gix-index` builds a `State` from a tree, not the reverse, so hashing the index as a tree means driving the tree editor from index entries or shelling out.
- Diff with rename detection: `diff_tree_to_tree` with `Options::track_rewrites` and `gix_diff::Rewrites`, including a similarity percentage (gix/src/object/tree/diff/mod.rs); merge options carry the same `rewrites` field.
- Merge without a worktree: `Repository::merge_trees`, `merge_commits`, `virtual_merge_base` (repository/merge.rs). The outcome holds "The ready-made (but unwritten) *base* tree, including all non-conflicting changes" as a tree editor, a `conflicts` vector, and `has_unresolved_conflicts(TreatAsUnresolved)` (gix/src/merge.rs), behind the `merge` feature.
- Worktrees: `Repository::worktrees() -> io::Result<Vec<worktree::Proxy>>`, `main_repo`, `is_bare` (repository/worktree.rs).
- Signed commits: `Commit::signature()` returns the signature and the signed data; `Commit::verify_signature()` resolves `gpg.format`, requires `gpg.ssh.allowedSignersFile` for SSH, and runs the configured program, so gix shells out to ssh-keygen or gpg for verification just as git does (gix/src/commit/verify.rs).
- Hash kinds: `gix_hash::Kind` has `Sha1` and `Sha256`; end-to-end SHA-256 repository support is unverified.

Pitfalls. The feature matrix is large and the defaults are broad; pick features explicitly. Minor versions break APIs on a monthly cadence, so pin exact versions. The merge is gitoxide's own implementation; equivalence with git's ORT merge on rename and directory-rename cases needs a spike (see Recommendations).

### git2 (libgit2)

Version 0.21.0, released 2026-05-18, MIT OR Apache-2.0, as of 2026-09-16 (https://crates.io/api/v1/crates/git2). The crates.io record carries no rust_version; the repository manifest for 0.21.0 declares `rust-version = "1.87"` and vendors libgit2 through libgit2-sys 0.18.7+1.9.6, and the README says "Currently this library requires libgit2 1.9.6 (or newer patch versions)" (https://github.com/rust-lang/git2-rs). Repository: 2,106 stars, 161 open issues and pull requests, last push 2026-09-02.

Fit. The API covers our list: `Repository::merge_trees`, `merge_base`, `worktrees`, `extract_signature`; `Index::write_tree_to`, `add_all`; `Diff::find_similar`; `MergeOptions::find_renames`, `rename_threshold`, `fail_on_conflict` (src on master).

Pitfalls. Three RustSec informational advisories in 2026, all "unsound" null-pointer slice issues: RUSTSEC-2026-0008 (patched >= 0.20.4), RUSTSEC-2026-0183 and RUSTSEC-2026-0184 (patched >= 0.21.0) (https://github.com/rustsec/advisory-db/tree/main/crates/git2). It is a C dependency with its own build and TLS toggles, and its merge is libgit2's own: `include/git2/merge.h` exposes `GIT_MERGE_FIND_RENAMES` and a threshold, and I found no directory-rename handling in `src/libgit2/merge.c` (https://github.com/libgit2/libgit2), whereas git's merge-tree manual lists directory rename conflicts. An integrated candidate built with libgit2 can differ from what `git merge` produces.

### The git binary

`git merge-tree --write-tree` arrived in Git 2.38.0: "git merge-tree learned a new mode where it takes two commits and computes a tree that would result in the merge commit" (https://github.com/git/git/blob/master/Documentation/RelNotes/2.38.0.adoc). Git 2.40.0 added `--merge-base` (https://github.com/git/git/blob/master/Documentation/RelNotes/2.40.0.adoc). From the current manual (https://github.com/git/git/blob/master/Documentation/git-merge-tree.adoc, as of 2026-09-16):

- Exit status: "For a successful, non-conflicted merge, the exit status is 0. When the merge has conflicts, the exit status is 1. If the merge is not able to complete (or start) due to some kind of error, the exit status is something other than 0 or 1 (and the output is unspecified)." With `--stdin` the status is 0 for both clean and conflicted merges.
- Output: the OID of the resulting toplevel tree, then for conflicts a "Conflicted file info" section of (mode, oid, stage, path) tuples and informational messages with stable conflict-type strings such as "CONFLICT (rename/delete)". Options: `-z`, `--name-only`, `--no-messages`, `--quiet`, `--allow-unrelated-histories`, `--merge-base=<tree-ish>`. With `--merge-base`, "<branch1> and <branch2> do not need to specify commits; trees are enough", which is the base-tree plus two lane-trees shape Warrant has.
- The usage notes warn: "Do NOT interpret an empty Conflicted file info list as a clean merge; check the exit status."

Other shell-outs that matter: `git worktree list --porcelain [-z]` is documented as "This format will remain stable across Git versions and regardless of user configuration" (https://github.com/git/git/blob/master/Documentation/git-worktree.adoc). `GIT_INDEX_FILE` "specifies an alternate index file" (https://github.com/git/git/blob/master/Documentation/git.adoc), so `GIT_INDEX_FILE=<tmp> git add -A && git write-tree` hashes the working tree as a tree without touching the real index or the worktree. Signature fields come through pretty formats `%G?`, `%GS`, `%GK`, `%GF` (https://github.com/git/git/blob/master/Documentation/pretty-formats.adoc).

### Recommendation

Use gix for all reads: objects by hash, tree diffs with rewrite tracking, worktree listing, and extracting a commit's signature and signed data. Use the git binary for `merge-tree --write-tree`, because the receipt must be able to say "this is the tree git would have produced" and only git's own ORT implementation can make that claim; require git 2.40 or later for `--merge-base` and record the git version in the receipt. Use the temporary-index write-tree shell-out for hashing the working tree until the gix tree-editor path is spiked. Do not take git2: the C build, the 2026 advisories, and the merge-semantics gap count against it, and nothing in our list needs it.

## 2. SQLite

### rusqlite

Version 0.40.2, released 2026-08-08, MIT, published by gwenn, as of 2026-09-16 (https://crates.io/api/v1/crates/rusqlite). The 0.40.2 manifest declares no rust-version; the master manifest (unreleased) declares `rust-version = "1.88.0"` and edition 2024 (https://github.com/rusqlite/rusqlite/blob/master/Cargo.toml). Repository: 4,400 stars, 168 open issues and pull requests, last push 2026-09-14. libsqlite3-sys 0.38.2 (2026-08-08, MIT) bundles SQLite 3.53.4: `upgrade.sh` fetches `sqlite-amalgamation-3530400` and the vendored `sqlite3.h` defines `SQLITE_VERSION "3.53.4"` (https://github.com/rusqlite/rusqlite/tree/master/libsqlite3-sys).

Bundled build flags (libsqlite3-sys/build.rs, as of 2026-09-16): `SQLITE_ENABLE_FTS3`, `SQLITE_ENABLE_FTS5`, `SQLITE_ENABLE_JSON1`, `SQLITE_ENABLE_RTREE`, `SQLITE_ENABLE_STAT4`, `SQLITE_ENABLE_COLUMN_METADATA`, `SQLITE_ENABLE_DBSTAT_VTAB`, `SQLITE_ENABLE_LOAD_EXTENSION=1`, `SQLITE_ENABLE_API_ARMOR`, `SQLITE_DEFAULT_FOREIGN_KEYS=1`, `SQLITE_THREADSAFE=1`. So FTS5 and JSON are both present with the `bundled` feature. JSON needs no flag anyway: "The JSON functions and operators are built into SQLite by default, as of SQLite version 3.38.0 (2022-02-22)" (https://www.sqlite.org/json1.html).

Relevant Cargo features (rusqlite/Cargo.toml): `bundled`, `functions`, `hooks` (authorizer, progress handler), `limits`, `trace`, `vtab`, `series`, `backup`, `blob`, `session` and `preupdate_hook`, `modern_sqlite`.

Read-only query surface. Everything needed to expose SQL to agents safely is present:

- `Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)` (src/lib.rs). Combine with a URI filename and `immutable=1`, which "signals to SQLite that the underlying database file is held on read-only media and cannot be modified" (https://www.sqlite.org/uri.html), for sealed snapshot files.
- `Connection::authorizer(Some(|ctx: AuthContext| -> Authorization))` under the `hooks` feature, with an `AuthAction` enum covering each SQLite authorizer action (src/hooks/mod.rs). Deny everything except `Select`, `Read`, and `Function`; deny `Attach`, `Pragma`, `LoadExtension`, and all DDL and DML.
- `set_db_config` with `SQLITE_DBCONFIG_DEFENSIVE` and `SQLITE_DBCONFIG_TRUSTED_SCHEMA` (src/config.rs), plus `PRAGMA query_only`, which "prevents data changes on database files when enabled" (https://www.sqlite.org/pragma.html#pragma_query_only).
- `set_limit` for `SQLITE_LIMIT_SQL_LENGTH`, `SQLITE_LIMIT_VDBE_OP` and `SQLITE_LIMIT_EXPR_DEPTH` (src/limits.rs), `progress_handler` for an instruction budget, and `get_interrupt_handle()` for a wall-clock timeout from another thread (src/hooks/mod.rs, src/lib.rs).

Advisories: RUSTSEC-2021-0128 (closure lifetime bounds, patched in 0.26.2) and libsqlite3-sys RUSTSEC-2022-0090 (CVE-2022-35737 in SQLite < 3.39.2, patched in 0.25.1) are both historical (https://github.com/rustsec/advisory-db/tree/main/crates/rusqlite).

One file per snapshot. WAL mode is for concurrent writers and adds `-wal` and `-shm` sidecars, which is wrong for a content-addressed artifact. Build the snapshot in rollback journal mode, `VACUUM` before sealing, then only open it read-only and immutable. File bytes depend on insertion order and page reuse, so if the receipt needs a database digest, digest a canonical logical dump rather than the file.

### Alternatives

sqlx 0.9.0, released 2026-05-21, MIT OR Apache-2.0, MSRV 1.94, as of 2026-09-16 (https://crates.io/api/v1/crates/sqlx). The GitHub repository now redirects to https://github.com/transact-rs/sqlx (754 open issues and pull requests, last push 2026-09-14). The SQLite driver depends on libsqlite3-sys `>=0.30.1, <0.39.0` (sqlx-sqlite/Cargo.toml). It is async-first ("Truly Asynchronous. Built from the ground-up using async/await"), which is a runtime and compile-time cost with no benefit for a single-process CLI writing one file per snapshot. Not recommended.

sqlite 0.37.0 (2025-03-28) is a thin wrapper without an authorizer surface that I could find. turso 0.7.2, with 0.8.0-pre.11 on 2026-09-11 (MIT), is the Rust rewrite of SQLite, pre-1.0 and unsuitable for artifacts that stock SQLite must open.

### Recommendation

rusqlite 0.40 with features `bundled`, `hooks`, `limits`, `functions`. Pin the bundled SQLite version in the receipt as an instrument version. Expose SQL through one hardened connection factory: read-only plus immutable open, authorizer allowlist, defensive and untrusted-schema config, limits, a progress-handler budget, and an interrupt on timeout.

## 3. Structural pattern matching

### tree-sitter

Version 0.27.0, released 2026-08-30, MIT, MSRV 1.90, as of 2026-09-16 (https://crates.io/api/v1/crates/tree-sitter). Repository: 26,973 stars, 112 open issues and pull requests, last push 2026-09-16. The companion crate tree-sitter-language 0.1.8 (2026-08-30, MIT, MSRV 1.90) supplies the `LanguageFn` type that grammar crates export; `tree_sitter::Language::new(LanguageFn)` and `From<LanguageFn>` are the bridge (lib/binding_rust/lib.rs at v0.27.0).

ABI coupling. The 0.27.0 header defines `TREE_SITTER_LANGUAGE_VERSION 15` and `TREE_SITTER_MIN_COMPATIBLE_LANGUAGE_VERSION 13` (https://github.com/tree-sitter/tree-sitter/blob/v0.27.0/lib/include/tree_sitter/api.h), and the Rust binding checks a grammar's ABI against that range at load time (lib/binding_rust/lib.rs). Grammar crates depend only on tree-sitter-language "0.1", with the runtime crate in dev-dependencies, so Cargo does not force lockstep; what matters is the ABI number compiled into each grammar's `parser.c`, as of 2026-09-16:

| Grammar crate | Version | Released | ABI in parser.c |
| --- | --- | --- | --- |
| tree-sitter-typescript (typescript and tsx) | 0.23.2 | 2024-11-11 | 14 |
| tree-sitter-javascript | 0.25.0 | 2025-09-01 | 15 |
| tree-sitter-rust | 0.24.2 | 2026-03-27 | 15 |
| tree-sitter-python | 0.25.0 | 2025-09-11 | 15 |
| tree-sitter-sequel (DerekStride/tree-sitter-sql) | 0.3.11 | 2025-10-01 | 15 |

All are MIT and all load under 0.27.0. The TypeScript grammar is the stale one: 22 months since release and 53 open issues and pull requests (last push 2026-09-13). tree-sitter-sql 0.0.2 (2021) is dead; tree-sitter-sequel is the SQL option if ever wanted.

Pitfalls. A grammar regenerated with a newer CLI can carry ABI 16 before the runtime supports it, and the failure is a runtime `LanguageError`, not a compile error. Pin every grammar exactly, load all of them in a startup self-test, and record each grammar version in the receipt as an instrument version.

### ast-grep as a library

ast-grep 0.45.3, released 2026-08-31, MIT, MSRV 1.88, edition 2024, published by HerringtonDarkholme, as of 2026-09-16; the crates ast-grep-core, ast-grep-config, ast-grep-language, and ast-grep-dynamic are all released together at 0.45.3 (https://crates.io/api/v1/crates/ast-grep-core and siblings). Repository https://github.com/ast-grep/ast-grep: 15,927 stars, 54 open issues and pull requests, last push 2026-09-16. The workspace pins tree-sitter 0.27.0, schemars 1.0, and serde_yaml 0.9.33 (https://github.com/ast-grep/ast-grep/blob/main/Cargo.toml). ast-grep-language pins tree-sitter-typescript 0.23.2, tree-sitter-javascript 0.25.0, tree-sitter-python 0.25.0, and tree-sitter-rust 0.24.0, matching the table above, and builds in 27 languages including TypeScript, Tsx, JavaScript, Python, and Rust but not SQL (crates/language/src/lib.rs). ast-grep-dynamic loads additional grammars from shared libraries.

Is library use supported? The official guide says: "Rust: ast-grep's Rust API is the most efficient way, but also the most challenging way, to use ast-grep. You can refer to ast_grep_core if you are familiar with Rust." (https://github.com/ast-grep/ast-grep.github.io/blob/main/website/guide/api-usage.md). There is no stability promise; the crates are 0.x and every release bumps all of them. The entry point for YAML rules is `ast_grep_config::from_yaml_string(yamls, &GlobalRules) -> Result<Vec<RuleConfig<L>>, RuleConfigError>` (crates/config/src/lib.rs), and the rule document keys are `id`, `language`, `rule`, `constraints`, `utils`, `fix`, `message`, `severity`, `note`, `files`, `ignores`, `url`, `metadata`, `labels`, `transform`, `rewriters` (https://ast-grep.github.io/reference/yaml.html). The `metadata` key is where Warrant's contract fields (intent, owner, ruling, enforcement, blind spots) can ride without forking the format.

Pitfalls. ast-grep-config depends on the archived serde_yaml 0.9.33, so that crate stays in the lock file transitively (it carries no current advisory). `RuleConfig<L>` needs a `Language` implementation; ast-grep-language's `SupportLang` is the easy path and ties our grammar set to theirs. The alternative is shelling out to the ast-grep binary, which gives isolation and a stable JSON contract at the cost of a runtime dependency.

### Recommendation

Use ast-grep-core, ast-grep-config, and ast-grep-language at an exact pin for pattern-enforced contracts, so that a contract is a real ast-grep rule with Warrant metadata and matches are available in process with metavariable bindings. Treat the ast-grep crate version as an instrument version in the receipt and run the frozen-corpus before-and-after report on every bump, exactly as the vision requires for analyzers. Keep the tree-sitter runtime at the version ast-grep pins (0.27.0 today), never newer.

## 4. Canonical JSON and hashing

### RFC 8785 (JCS) crates

serde_jcs 0.2.0, released 2026-03-25, MIT OR Apache-2.0, MSRV 1.85, as of 2026-09-16 (https://crates.io/api/v1/crates/serde_jcs). Repository https://github.com/l1h3r/serde_jcs: 8 stars, 1 open issue, last push 2026-03-25; its main-branch manifest still reads version 0.1.0 with ryu-js 0.2 and serde_json `float_roundtrip`, so the published 0.2.0 source should be read from the crate itself, not the repository. The README states only that it implements "JSON Canonicalization Scheme (JCS) for Serde" and cites RFC 8785.

serde_json_canonicalizer 0.3.2, released 2026-02-03, MIT, as of 2026-09-16 (https://crates.io/api/v1/crates/serde_json_canonicalizer). Repository https://github.com/evik42/serde-json-canonicalizer: 21 stars, 3 open issues, last push 2026-02-20. Depends on ryu-js 1.0.1 and serde_json with `float_roundtrip`; exposes `to_string` and `to_vec` as "drop-in replacement for serde_json" and describes itself as "An RFC 8785 compatible JSON Canonicalization Scheme output for serde_json" (README). json-canon 0.1.3 (2023-05-13, Apache-2.0) has had no release in three years.

Fit and pitfalls. Both are tiny single-maintainer crates that get the hard parts of JCS right by construction: ES6 number formatting through ryu-js and sorted keys. RFC 8785 sorts keys by UTF-16 code units, which differs from byte order outside the Basic Multilingual Plane, so a hand-rolled canonicalizer on a `BTreeMap` is subtly wrong. Warrant's schemas use ASCII keys and should forbid non-integer numbers in signed documents, which makes the choice low-risk. Recommendation: serde_json_canonicalizer 0.3 behind one `canonical_bytes(&T)` function, with the official JCS test vectors (https://github.com/cyberphone/json-canonicalization) as fixtures.

### Hash functions

sha2 0.11.0, released 2026-03-25, MIT OR Apache-2.0, MSRV 1.85, on digest 0.11.3, as of 2026-09-16 (https://crates.io/api/v1/crates/sha2, https://crates.io/api/v1/crates/digest). The 0.11 line is a breaking change from 0.10 and the ecosystem is split today: ssh-key 0.6.7 depends on sha2 0.10.8 while ssh-key 0.7.0-rc.11 depends on sha2 0.11 (manifests at the respective tags in https://github.com/RustCrypto/SSH). Choosing sha2 0.11 with ssh-key 0.6 compiles two sha2 versions; harmless but worth knowing.

blake3 1.8.7, released 2026-08-20, CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception, as of 2026-09-16 (https://crates.io/api/v1/crates/blake3). `Hasher::update_mmap_rayon` (features `mmap` and `rayon`) hashes large files with memory mapping and a thread pool (src/lib.rs). Repository: 6,438 stars, 201 open issues and pull requests, last push 2026-09-10.

Digest formats. The OCI descriptor grammar is `algorithm:encoded`, for example `sha256:6c3c624b...`, and "compliant implementations SHOULD use SHA-256" (https://github.com/opencontainers/image-spec/blob/main/descriptor.md). in-toto's ResourceDescriptor carries a DigestSet map, `"digest": { "<ALGORITHM>": "<HEX VALUE>" }` (https://github.com/in-toto/attestation/blob/main/spec/v1/resource_descriptor.md). multihash 0.19.5 (2026-04-27, MIT, MSRV 1.81) and multibase 0.9.3 (2026-06-29, MIT) implement the self-describing formats, where sha2-256 is code 0x12 and blake3 is 0x1e and still marked draft (https://github.com/multiformats/multicodec/blob/master/table.csv).

Recommendation. One external digest algorithm, SHA-256, rendered as `sha256:<64 lowercase hex>` in every CLI and JSON surface and as a DigestSet inside in-toto statements, so a receipt can be checked with `sha256sum`. Git object ids stay whatever the repository uses and are labeled as such, never mixed into the digest namespace. blake3 is optional for internal cache keys; never put a blake3 value in a receipt. Skip multihash and multibase; nothing downstream consumes CIDs.

## 5. Signatures and attestations

### ssh-key and sshsig

ssh-key stable is 0.6.7, Apache-2.0 OR MIT, with `rust-version = "1.65"` in its manifest; the newest release is 0.7.0-rc.11 from 2026-06-29 with MSRV 1.85 and edition 2024, as of 2026-09-16 (https://crates.io/api/v1/crates/ssh-key, manifests at https://github.com/RustCrypto/SSH). Repository: 246 stars, 14 open issues and pull requests, last push 2026-09-14, maintained inside the RustCrypto organization.

SshSig support in 0.6.7 (ssh-key/src/sshsig.rs at tag ssh-key/v0.6.7): `SshSig::new`, `from_pem`, `to_pem`, `sign(signing_key, namespace, hash_alg, msg)`, `signed_data`; verification is `PublicKey::verify(&self, namespace, msg, &SshSig)` (ssh-key/src/public.rs). 0.7.0-rc.11 adds prehash variants and rejects empty namespaces explicitly. Algorithms are feature-gated: `ed25519` (ed25519-dalek 2 in 0.6.7, 3 in 0.7), `rsa`, `p256`, `p384`, `p521`. ed25519-dalek itself is at 3.0.0, released 2026-07-06, BSD-3-Clause, MSRV 1.85 (https://crates.io/api/v1/crates/ed25519-dalek).

What ssh-key does not have: an allowed_signers parser. The source tree contains `authorized_keys.rs`, `known_hosts.rs`, and `dot_ssh.rs` but nothing for allowed signers, and a code search for the term returned nothing (https://github.com/RustCrypto/SSH/tree/master/ssh-key/src). The format is small. Per the OpenSSH manual, "Each line of the file contains the following space-separated fields: principals, options, keytype, base64-encoded key", principals are a comma-separated pattern list of USER@DOMAIN identities, and options include `cert-authority`, `namespaces="namespace-list"`, `valid-after="timestamp"`, and `valid-before` (https://github.com/openssh/openssh-portable/blob/master/ssh-keygen.1, ALLOWED SIGNERS section). The key field parses with ssh-key's `PublicKey::from_openssh`.

ssh-keygen. `ssh-keygen -Y sign -f key -n namespace file` produces the signature and `ssh-keygen -Y verify -f allowed_signers_file -I signer_identity -n namespace -s signature_file [-r revocation_file]` checks it; the manual recommends "names following a NAMESPACE@YOUR.DOMAIN pattern to generate unambiguous namespaces" (https://man.openbsd.org/ssh-keygen.1).

Recommendation. Humans sign rulings with `ssh-keygen -Y sign`, so private keys never touch Warrant. Verify in process with ssh-key, because verification runs on every evaluation and must not depend on PATH or the OpenSSH version on a CI runner; hand-roll the allowed_signers parser (principals, `namespaces=`, validity window, and a rejection of `cert-authority` for v1) and enforce a fixed namespace such as `warrant-ruling@<your-domain>` so a signature made for another purpose cannot be replayed. Keep `ssh-keygen -Y verify` as the interop oracle in the test suite so both paths must agree on the same fixtures. Take 0.6.7 for day one; move to 0.7 when it leaves release candidate, and isolate the dependency behind a small `Verifier` trait so that swap and a future algorithm change are local.

### in-toto Statement v1 and DSSE

in-toto 0.4.0, released 2024-12-11, MIT, as of 2026-09-16 (https://crates.io/api/v1/crates/in-toto). Repository https://github.com/in-toto/in-toto-rs: 35 stars, last push 2026-07-28, and the 2026 commits are dependabot merges. It has a DSSE `envelope` module, but its statement module only knows `https://in-toto.io/Statement/v0.1`, not v1 (src/models/statement/mod.rs), and it depends on ring 0.17. Not a fit.

There is no canonical `dsse` crate: the name does not exist on crates.io, and a search returns small application-specific crates (wsc-dsse, mkit-attest, kobold-attest, agent-toolprint), none with a community behind them. sigstore 0.14.0 (2026-05-22, Apache-2.0) ships an in-toto bundle module (src/bundle/intoto.rs) but brings the whole Sigstore client with it (fulcio, rekor, oauth, registry); 75 open issues and pull requests (https://github.com/sigstore/sigstore-rs).

The specifications are short. DSSE: `PAE(type, body) = "DSSEv1" + SP + LEN(type) + SP + type + SP + LEN(body) + SP + body`, signatures are computed over `PAE(UTF8(PAYLOAD_TYPE), SERIALIZED_BODY)`, and the envelope is `{"payload": <base64>, "payloadType": <string>, "signatures": [{"keyid": <string>, "sig": <base64>}]}` (https://github.com/secure-systems-lab/dsse/blob/master/protocol.md). Statement v1: `_type` is `https://in-toto.io/Statement/v1`, `subject` is an array of ResourceDescriptor objects, then `predicateType` and `predicate` (https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md).

Recommendation. Hand-roll both. A Statement v1 struct, a DSSE envelope struct, and the PAE function are under a hundred lines with serde, and the signature inside the envelope is an sshsig over the PAE bytes with `keyid` set to the signer's SHA256 fingerprint. That keeps rulings and receipts readable by any in-toto tooling that accepts a custom signature scheme, without inheriting a stale crate or the Sigstore client.

## 6. SARIF and JSON Schema

### serde-sarif

Version 0.8.0, released 2025-05-09, MIT, edition 2018, as of 2026-09-16 (https://crates.io/api/v1/crates/serde-sarif). Repository https://github.com/psastras/sarif-rs (workspace with clippy-sarif, shellcheck-sarif, sarif-fmt and others): 135 stars, 12 open issues, last push 2026-09-14, last release 2025-05-09. The types are generated at build time from the bundled SARIF 2.1.0 `schema.json` with schemafy_lib, with a hand workaround for `PropertyBag` (serde-sarif/build.rs), and an `opt-builder` feature adds builder setters. Coverage is therefore the whole 2.1.0 schema, and the price is schemafy's shape: nearly every field is an `Option`, and the crate implements no `JsonSchema`.

Fit. Warrant consumes SARIF from external instruments and emits SARIF for its own findings; serde-sarif serves both if wrapped: parse instrument output as `serde_json::Value` first to record the driver name, version, and file digest for the receipt, then into `serde_sarif::sarif::Sarif`. The sixteen-month release gap is the risk; the code is generated from a frozen schema, so drift is unlikely. A minimal reader for `runs[].results[]` with `ruleId`, `level`, `message.text`, and `locations[].physicalLocation` is about a page of code and a reasonable fallback.

### schemars

Version 1.2.2, released 2026-07-27, MIT, MSRV 1.74, as of 2026-09-16 (https://crates.io/api/v1/crates/schemars). Repository: 1,411 stars, 119 open issues and pull requests, last push 2026-07-27. The README states "Schemars aims to support the past year of stable rust versions" and that an MSRV increase is a semver-minor change. Generated schemas default to JSON Schema draft 2020-12 with draft-07 and OpenAPI 3.0 available through `SchemaSettings`, and "Schemars will check for any `#[serde(...)]` attributes on types that derive `JsonSchema`, and adjust the generated schema accordingly" (https://github.com/GREsau/schemars/blob/master/README.md). ast-grep pins schemars 1.0, so one major version serves both.

Pitfalls. The predecessor is on schemars 0.8, and 1.x changed the `Schema` type to a thin wrapper over `serde_json::Value`; the migration is mechanical but real. Versioned CLI output schemas should be generated in a build step, written next to the code, and snapshot-tested with insta 1.48.0 (2026-06-11, Apache-2.0) so a schema change is a reviewed diff, and each output document should carry a `schema_version` field the way the predecessor's npm snapshot already does.

## 7. Rule and policy engines

ascent 0.8.1, released 2026-08-29, MIT, MSRV 1.85, as of 2026-09-16 (https://crates.io/api/v1/crates/ascent). Repository https://github.com/s-arash/ascent: 582 stars, 10 open issues and pull requests. A macro-generated Datalog with semi-naive evaluation, user-defined lattices, and parallel evaluation through `ascent_par!` on rayon (README). It has no incremental maintenance: a changed fact means a rerun of the program.

crepe 0.2.0, released 2025-12-14, MIT OR Apache-2.0, as of 2026-09-16 (https://crates.io/api/v1/crates/crepe). Repository https://github.com/ekzhang/crepe: 531 stars, 6 open issues, last push 2025-12-14. A procedural macro with "Semi-naive evaluation" and "Stratified negation" (README); one maintainer, slow cadence, no incremental support. datafrog 2.0.1 (2019) is frozen. egglog 3.0.0 (2026-08-19, MIT) is an e-graph engine and solves a different problem.

cedar-policy 4.13.0, released 2026-09-15, Apache-2.0, MSRV 1.89, as of 2026-09-16 (https://crates.io/api/v1/crates/cedar-policy). Repository: 1,737 stars, 175 open issues and pull requests, last push 2026-09-16. Its analysis lives in cedar-policy-symcc 0.7.0, released 2026-09-15, which offers `check_never_errors`, `check_always_allows`, `check_always_denies`, `check_implies` (policy set subsumption), `check_equivalent`, `check_disjoint`, each with a counterexample variant, is "formally modeled and verified in Lean", and requires the external cvc5 1.3.1 SMT solver and an async runtime (https://github.com/cedar-policy/cedar/blob/main/cedar-policy-symcc/README.md). For a policy written in Cedar, `check_implies(new, old)` is exactly the "did this change widen what is permitted" question, with a synthesized request as the witness.

Honest recommendation for v1: plain Rust over petgraph. Every contract kind in the vision (ownership, a single permitted route, dependency direction, capability obligations, pattern rules) is a reachability or set-membership query over the program model, and each finding must carry "the shortest import or registration path that proves it" plus the enforcement's blind spots; explicit code keeps that provenance attached to each rule. Neither ascent nor crepe is incremental, and neither gives path provenance for free. Cedar's widening analysis is real and verified, but only for policies written in Cedar over a Cedar schema, and it adds an SMT solver to the install. Implement the widening classifier structurally over Warrant's own policy AST with the vision's rule that "we couldn't tell" is widening, and revisit Cedar only if a permission-shaped contract family is added later.

## 8. Graphs

petgraph 0.8.3, released 2025-09-30, MIT OR Apache-2.0, crates.io MSRV 1.64, as of 2026-09-16 (https://crates.io/api/v1/crates/petgraph). The master workspace declares `rust-version = "1.91"` for the next release. Repository: 4,016 stars, 292 open issues and pull requests, last push 2026-09-13; the crate is now a workspace of `petgraph` and `petgraph-core`. Algorithms present on master (crates/petgraph/src/algo): `tarjan_scc`, `kosaraju_scc`, `condensation`, `dominators::simple_fast` with `immediate_dominator`, `strict_dominators`, `dominators`, `immediately_dominated_by`; `dijkstra`, `bidirectional_dijkstra`, `astar`, `bellman_ford`, `floyd_warshall`, `johnson`, `k_shortest_path`, `all_simple_paths`, `spfa`; `toposort`, `is_cyclic_directed`, `has_path_connecting`, `connected_components`, `greedy_feedback_arc_set`, `bridges`, `articulation_points`, `maximal_cliques`, `page_rank`, minimum spanning tree, maximum flow, Steiner tree, isomorphism.

Alternatives. rustworkx-core 0.18.1 (2026-07-29, Apache-2.0, Qiskit) adds algorithms but is built on petgraph, so it is an extension rather than a replacement. fixedbitset 0.5.7 is petgraph's own dependency.

Fit and pitfalls. The predecessor is on 0.7 and 0.8 is a straightforward bump. Use `StableGraph` or a `GraphMap` keyed by stable node ids so that removals do not renumber, and sort every output (SCC members, paths, condensation nodes) by the stable key, because petgraph iteration order follows insertion order and a receipt must not depend on walk order. `condensation` plus `toposort` gives layer and cycle findings; `dominators` answers "every path to X goes through Y" for single-entrypoint questions; unit-weight `dijkstra` or a BFS produces the proof path for a finding.

## 9. CLI and output

clap 4.6.7, released 2026-09-14, MIT OR Apache-2.0, MSRV 1.85 (https://crates.io/api/v1/crates/clap). miette 7.6.0, released 2025-04-27, Apache-2.0, MSRV 1.70, 115 open issues and pull requests, last push 2026-06-25 (https://crates.io/api/v1/crates/miette); the `fancy` feature is for human terminals only, and machine output must never pass through it. thiserror 2.0.20 (2026-08-08, MSRV 1.71) and anyhow 1.0.104 (2026-07-18). serde 1.0.229 and serde_json 1.0.151 (2026-07-20, MSRV 1.71); `preserve_order` is harmless but canonical output sorts keys anyway.

YAML. serde_yaml's final release is 0.9.34+deprecated from 2024-03-25 and the repository is archived. The predecessor's replacement, serde_yml 0.0.13, is worse: its repository is archived and RUSTSEC-2025-0068 (2025-09-11) is titled "serde_yml crate is unsound and unmaintained", because "Using `serde_yml::ser::Serializer.emitter` can cause a segmentation fault" (https://github.com/rustsec/advisory-db/blob/main/crates/serde_yml/RUSTSEC-2025-0068.md). It must go. Candidates as of 2026-09-16: serde_yaml_ng 0.10.0 (2024-05-26, MIT, MSRV 1.64, repository last push 2025-09-14), serde_norway 0.9.42 (2024-12-21, MSRV 1.71.1), and serde-saphyr 1.3.0 (2026-09-16, MIT OR Apache-2.0, MSRV 1.89, 221 stars, 1 open issue). serde-saphyr is pure Rust on the saphyr parser, has "Configurable budgets: Enforce input limits to mitigate resource exhaustion" and a `DuplicateKeyPolicy`, and tests its MSRV in CI (crates.io README). Recommendation: serde-saphyr with budgets set and duplicate keys rejected, because contract files are agent-written input and the parser is part of the trust boundary.

Logging and progress. tracing 0.1.44 (2025-12-18) and tracing-subscriber 0.3.23 (2026-03-13), MSRV 1.65. indicatif 0.18.6 (2026-07-01, MIT, MSRV 1.85) hides itself when stderr is not a terminal or TERM is dumb: `if !term.is_term() || is_dumb() { return Self::hidden(); }` (https://github.com/console-rs/indicatif/blob/main/src/draw_target.rs). Progress messages are free text the caller sets, so the rule for not leaking secrets is a coding rule: messages carry counts and repository-relative paths, never environment values, command lines, or file contents, and a `--no-progress` flag exists for CI logs.

Cancellation. `CommandExt::process_group` has been stable since Rust 1.64 (https://github.com/rust-lang/rust/blob/master/library/std/src/os/unix/process.rs). ctrlc 3.5.2 (2026-02-10, MIT/Apache-2.0, MSRV 1.69) plus an `AtomicBool` and a registry of child process groups to kill is enough for a synchronous CLI. tokio 1.53.1 (2026-07-20, MSRV 1.71) with tokio-util 0.7.19's `CancellationToken` is the right tool only if the instrument runner becomes async; keep tokio out of the core crate for v1. rayon 1.12.0 (2026-04-14, MSRV 1.80): build a scoped pool sized by a `--jobs` flag rather than using the global pool, so blake3's rayon feature does not compete with it.

## 10. Filesystem and inventory

ignore 0.4.33, released 2026-08-04, Unlicense OR MIT, MSRV 1.88 (https://crates.io/api/v1/crates/ignore), from the ripgrep workspace (68,336 stars, last push 2026-08-04). `WalkBuilder` exposes `follow_links`, `same_file_system`, `ignore_case_insensitive`, `hidden`, `git_ignore`, `git_global`, `git_exclude`, `require_git`, `threads` with `build_parallel`, `max_filesize`, `sort_by_file_path`, `filter_entry`, and `add_custom_ignore_filename` (https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/walk.rs). globset 0.4.20 (2026-08-04, MSRV 1.88) is the same workspace. walkdir 2.5.0 (2024-03-01) is stable and has no ignore semantics. jwalk 0.9.0 (2026-08-05, MIT) is a parallel walker; not needed if `build_parallel` is used.

Inventory semantics. gitignore matching is not the same as git's tracked set; a tracked file that matches an ignore pattern is still tracked. The inventory should start from the snapshot tree (tracked entries from gix), add untracked and not-ignored files from the walk, and record for each file which source put it there, which is the "classification and a reason" the vision requires. Hidden files must be included (`hidden(false)`) or the denominator is wrong.

Hashing at scale. Reuse git blob ids for tracked files that are clean against the index and rehash the rest, with sha2 for external digests. mtime is a hint, never evidence: git's design note explains that index entries whose mtime equals the index write time are "racily clean" and must be re-checked, and that nanosecond resolution "is not stable on network filesystems" (https://github.com/git/git/blob/master/Documentation/technical/racy-git.adoc). A size-plus-mtime cache is fine if any entry whose mtime is at or after the snapshot start is rehashed.

Symlinks and case. Git stores a symlink as mode `120000` with "the content of the file will be the link target" (https://github.com/git/git/blob/master/Documentation/git-fast-import.adoc); the inventory records the link target and never follows it, which is `ignore`'s default. `core.ignoreCase` exists because on "filesystems that are not case sensitive, like APFS, HFS+, FAT, NTFS" git will treat "makefile" and "Makefile" as the same file (https://github.com/git/git/blob/master/Documentation/config/core.adoc); Warrant should detect paths in a snapshot that collide case-insensitively and report them as a finding instead of normalizing anything. Paths are bytes in git; carry them as `BString` from gix rather than `String`.

## 11. Process limits for external instruments

Crates as of 2026-09-16: nix 0.31.3 (2026-05-11, MIT, MSRV 1.69) exposes `sys::resource::setrlimit` with `RLIMIT_AS`, `RLIMIT_CPU`, `RLIMIT_RSS` and the rest (https://github.com/nix-rust/nix/blob/master/src/sys/resource.rs); rlimit 0.11.0 (2026-02-01, MIT, MSRV 1.65) is a smaller wrapper for the same calls; process-wrap 10.0.0 (2026-08-24, Apache-2.0 OR MIT, MSRV 1.87), the successor to command-group, provides composable wrappers for process groups, sessions, job objects, and kill-on-drop with a std or tokio frontend, and states "Only the latest stable rustc version is supported" (https://github.com/watchexec/process-wrap); wait-timeout 0.2.1 (2025-02-03) adds `wait_timeout` to `Child`; cgroups-rs 0.5.1 (2026-07-14, kata-containers) needs cgroup write access, which an unprivileged process only has inside a delegated systemd user slice.

Linux. Apply limits in the child through `Command::pre_exec`: `RLIMIT_CPU` for CPU seconds, `RLIMIT_AS` for address space, `RLIMIT_FSIZE` and `RLIMIT_NOFILE` for hygiene, plus `process_group(0)` so a wall-clock timeout can kill the whole tree. Where systemd is present, `systemd-run --user --scope -p MemoryMax= -p CPUQuota= -p TasksMax=` gives cgroup v2 accounting without root (https://github.com/systemd/systemd/blob/main/man/systemd.resource-control.xml, systemd-run.xml); treat it as optional and record whether it was used.

macOS. The xnu `getrlimit(2)` manual lists `RLIMIT_CORE`, `CPU`, `DATA`, `FSIZE`, `MEMLOCK`, `NOFILE`, `NPROC`, `RSS`, `STACK` and does not list `RLIMIT_AS` (https://github.com/apple-oss-distributions/xnu/blob/main/bsd/man/man2/getrlimit.2), while `bsd/kern/kern_resource.c` on the same branch does handle `RLIMIT_AS` by calling `vm_map_set_size_limit`. Whether that limit is enforced on allocation on shipping macOS versions is unverified and is a spike. Plan on wall-clock plus `RLIMIT_CPU` everywhere, `RLIMIT_AS` on Linux, and a polling RSS watchdog on macOS.

Capture. Read stdout and stderr through pipes into files while hashing them with sha2, cap the byte count, and record the digest, the size, the exit status, the signal if any, the wall and CPU time, and the limits actually applied in the receipt; a truncated stream is a failure per the vision, not a warning. `RLIMIT_FSIZE` only bounds files the child writes itself, so the pipe reader is the real cap.

## 12. Packaging

cargo-dist (now "dist") 0.32.0 on crates.io, released 2026-05-22, MIT OR Apache-2.0, MSRV 1.74, as of 2026-09-16 (https://crates.io/api/v1/crates/cargo-dist); the repository has a v0.33.0 tag from 2026-09-11 that is not on crates.io yet (https://github.com/axodotdev/cargo-dist: 2,115 stars, 329 open issues and pull requests, last push 2026-09-13). It generates the GitHub Actions release pipeline, archives, shell and PowerShell installers, a Homebrew formula, and an npm installer package that "will fetch your prebuilt archives and install your binaries to node_modules" (book/src/installers/npm.md). Checksums default to `sha256` (`checksum` in book/src/reference/config.md) and `github-attestations` (since 0.16.0, still marked experimental) wires GitHub artifact attestations, which are Sigstore-backed; the action behind that is actions/attest-build-provenance v4.2.2 (2026-08-06). Alternatives: GoReleaser builds Rust "Since v2.5" through cargo zigbuild (https://goreleaser.com/customization/builds/rust/); cargo-release 1.1.6 and release-plz 0.3.167 handle versioning and changelogs; cargo-binstall 1.23.0 installs from release archives declared in package metadata. For signing independent of GitHub, minisign 0.9.1 and minisign-verify 0.2.5 (2026-03-03, MIT) sign and verify a `SHA256SUMS` file with an offline key.

npm wrapper. The predecessor ships one package whose launcher searches `SPECGATE_NATIVE_BIN`, then `native/<platform>/<arch>/`, then `native/<platform>/`, and relays the child's exit status or signal (npm/specgate/bin/specgate.js); one fat package means every install carries every platform's binary. The pattern to adopt is per-platform packages listed as `optionalDependencies` with `os` and `cpu` fields, as esbuild and Biome do, plus a launcher that verifies the binary's sha256 against a manifest embedded at publish time. dist's npm installer downloads at install time and, as far as its documentation states, verifies only the checksum.

MSRV policy. Dependency floors as of 2026-09-16: tree-sitter 0.27 needs 1.90, ignore and globset 1.88, ast-grep 1.88, serde-saphyr and cedar 1.89, gix 1.85, rusqlite master 1.88, oxc_parser 0.150.0 needs 1.96 and oxc_resolver 11.24.3 needs 1.95, sqlx 1.94. Stable is 1.98.1. Set `rust-version = "1.90"` for the core crate on day one, or 1.96 if oxc lives in the same crate; bump the MSRV only in minor releases and never above stable minus two. Edition 2024 selects Cargo resolver 3, which sets `resolver.incompatible-rust-versions` to `fallback` so resolution prefers MSRV-compatible versions (https://doc.rust-lang.org/cargo/reference/resolver.html); enforce it with a CI job on the pinned MSRV toolchain.

## Recommendations for the spec

Day-one pin list, exact versions as of 2026-09-16:

| Concern | Crate | Version | License |
| --- | --- | --- | --- |
| Git reads, diff, worktrees, signatures | gix | 0.87.1 | MIT OR Apache-2.0 |
| Snapshot store | rusqlite (bundled, hooks, limits, functions) | 0.40.2 | MIT |
| Pattern contracts | ast-grep-core, ast-grep-config, ast-grep-language | 0.45.3 | MIT |
| Parser runtime | tree-sitter | 0.27.0 | MIT |
| Canonical JSON | serde_json_canonicalizer | 0.3.2 | MIT |
| Digests | sha2 | 0.11.0 | MIT OR Apache-2.0 |
| Ruling verification | ssh-key (ed25519, and rsa or ecdsa as required) | 0.6.7 | Apache-2.0 OR MIT |
| SARIF | serde-sarif | 0.8.0 | MIT |
| Output schemas | schemars | 1.2.2 | MIT |
| Graph model | petgraph | 0.8.3 | MIT OR Apache-2.0 |
| CLI | clap | 4.6.7 | MIT OR Apache-2.0 |
| Diagnostics | miette, thiserror | 7.6.0, 2.0.20 | Apache-2.0; MIT OR Apache-2.0 |
| JSON | serde, serde_json | 1.0.229, 1.0.151 | MIT OR Apache-2.0 |
| YAML | serde-saphyr | 1.3.0 | MIT OR Apache-2.0 |
| Walking and globs | ignore, globset | 0.4.33, 0.4.20 | Unlicense OR MIT |
| Logging and progress | tracing, tracing-subscriber, indicatif | 0.1.44, 0.3.23, 0.18.6 | MIT |
| Cancellation and parallelism | ctrlc, rayon | 3.5.2, 1.12.0 | MIT/Apache-2.0; MIT OR Apache-2.0 |
| Child processes | nix, process-wrap, wait-timeout | 0.31.3, 10.0.0, 0.2.1 | MIT; Apache-2.0 OR MIT; MIT/Apache-2.0 |
| Time | jiff or chrono | 0.2.37 or 0.4.45 | Unlicense OR MIT; MIT OR Apache-2.0 |
| Tests | insta, tempfile, pretty_assertions | 1.48.0, 3.27.0, 1.x | Apache-2.0; MIT OR Apache-2.0 |

Spikes before the spec freezes:

1. Integrated candidate equivalence. Run `git merge-tree --write-tree --merge-base=<base> <a> <b>` and `gix::Repository::merge_trees` over a corpus with renames, directory renames, mode changes, and binary conflicts, and compare tree ids and conflict sets. If they diverge, git is the only producer of integrated candidates and gix is a reader.
2. Working-tree and index hashing without the git binary. Build a tree from the index and from a dirwalk with gix's tree editor and compare against `git write-tree`; until this passes, the temporary-index shell-out is the specified path.
3. Ruling verification interop. Sign fixtures with `ssh-keygen -Y sign` across ed25519, ecdsa, and rsa keys, verify with ssh-key 0.6.7 and with `ssh-keygen -Y verify` against a hand-parsed allowed_signers file with `namespaces=` and validity windows, and confirm both reject the same tampered inputs.
4. Process limits on macOS. Measure whether `RLIMIT_AS` and `RLIMIT_CPU` bind a child spawned from Rust, and decide what the receipt records when a limit could not be applied.

Where shelling out beats a crate: `git merge-tree --write-tree` for the integrated candidate; `GIT_INDEX_FILE` plus `git write-tree` for working-tree hashing until spike 2 passes; `ssh-keygen -Y sign` for humans, always, since Warrant never holds a private key; and every external instrument (tsc, knip, Fallow, linters) by design, with version, arguments, limits, and output digests recorded.

## Verification notes

Versions, release dates, licenses, and crates.io MSRV fields were read from the crates.io API on 2026-09-16 for every crate named above. API claims were checked against the current default branch of each repository on the same day, and tagged manifests were used where a released version and master differ (git2 0.21.0, rusqlite 0.40.2, ssh-key 0.6.7, tree-sitter 0.27.0, and each grammar crate). Repository health figures are the GitHub API's `open_issues_count`, which combines issues and pull requests. Not verified: end-to-end SHA-256 repository support in gix, ORT equivalence of gix merges, RLIMIT_AS enforcement on macOS, whether dist's npm installer verifies more than a checksum, and the reason for gitoxide's low open-issue count.
