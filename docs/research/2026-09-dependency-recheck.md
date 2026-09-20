# Dependency recheck for the M0-M2 build

Status: completed 2026-09-20 for W0.2.

This recheck queried the crates.io API for the exact release metadata, resolved
the workspace with Cargo resolver 3, and ran `cargo deny check` against the
resulting locked graph. The versions below were current non-yanked stable
releases unless a row says otherwise. Exact pins live in
`[workspace.dependencies]`; crate manifests select only the dependencies their
planned M0-M2 work uses.

`gix` is accepted without its default performance features. Those defaults pull
`uluru` under MPL-2.0, which this repository's permissive-only `cargo-deny`
policy rejects. The selected `basic`, `dirwalk`, `sha1`, `sha256`, `status`, and
`worktree-mutation` features cover the planned snapshot operations without that
dependency. `miette` is accepted without `fancy` for the same policy reason:
`fancy` pulls the unapproved ISC-licensed `is_ci` crate. Human rendering remains
possible without the optional terminal decoration.

## Accepted Cargo pins

| Crate | Verified version | License | Planned owner |
| --- | --- | --- | --- |
| `gix` | 0.87.1 | MIT OR Apache-2.0 | snapshot |
| `rusqlite` | 0.40.2 | MIT | model |
| `oxc_parser` | 0.150.0 | MIT | lang-ts |
| `oxc_ast` | 0.150.0 | MIT | lang-ts |
| `oxc_semantic` | 0.150.0 | MIT | lang-ts |
| `oxc_span` | 0.150.0 | MIT | lang-ts |
| `oxc_allocator` | 0.150.0 | MIT | lang-ts |
| `oxc_resolver` | 11.24.3 | MIT | lang-ts |
| `tree-sitter-typescript` | 0.23.2 | MIT | core pattern checks |
| `cargo_metadata` | 0.23.1 | MIT | inventory and lang-rust |
| `syn` | 3.0.6 | MIT OR Apache-2.0 | lang-rust |
| `ast-grep-core` | 0.45.3 | MIT | core pattern checks |
| `ast-grep-config` | 0.45.3 | MIT | core pattern checks |
| `ast-grep-language` | 0.45.3 | MIT | core pattern checks |
| `tree-sitter` | 0.27.0 | MIT | core pattern checks |
| `serde_json_canonicalizer` | 0.3.2 | MIT | core, model, authority |
| `sha2` | 0.11.0 | MIT OR Apache-2.0 | digests |
| `serde-sarif` | 0.8.0 | MIT | evidence adapters |
| `schemars` | 1.2.2 | MIT | core schemas |
| `petgraph` | 0.8.3 | MIT OR Apache-2.0 | core and model graphs |
| `clap` | 4.6.7 | MIT OR Apache-2.0 | CLI |
| `miette` | 7.6.0 | Apache-2.0 | CLI diagnostics |
| `thiserror` | 2.0.20 | MIT OR Apache-2.0 | typed core errors |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | emitted documents |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | emitted documents |
| `serde-saphyr` | 1.3.0 | MIT OR Apache-2.0 | manifest and policy YAML |
| `ignore` | 0.4.33 | Unlicense OR MIT | snapshot and inventory walks |
| `globset` | 0.4.20 | Unlicense OR MIT | snapshot and inventory paths |
| `tracing` | 0.1.44 | MIT | CLI diagnostics |
| `tracing-subscriber` | 0.3.23 | MIT | CLI diagnostics |
| `indicatif` | 0.18.6 | MIT | terminal-only progress |
| `ctrlc` | 3.5.2 | MIT OR Apache-2.0 | cancellation |
| `rayon` | 1.12.0 | MIT OR Apache-2.0 | bounded parallel work |
| `nix` | 0.31.3 | MIT | child limits and signals |
| `process-wrap` | 10.0.0 | Apache-2.0 OR MIT | child process groups |
| `wait-timeout` | 0.2.1 | MIT OR Apache-2.0 | child deadlines |
| `jiff` | 0.2.37 | Unlicense OR MIT | timestamps |
| `insta` | 1.48.0 | Apache-2.0 | tests |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | isolated tests and writes |
| `pretty_assertions` | 1.4.1 | MIT OR Apache-2.0 | tests |

The OXC set still requires Rust 1.96 and `oxc_resolver` requires 1.95, matching
the `warrant-lang-ts` 1.96 floor. All other accepted direct pins are compatible
with their owning crate's declared 1.90 floor. `cargo deny check` reported
advisories, bans, licenses, and sources all OK for the locked graph.

## Deferred entries

| Entry | Verified candidate | Reason and adoption point |
| --- | --- | --- |
| `scip` | 0.10.0, Apache-2.0 | Optional; only W1.5/W1.D10 may adopt it if the SCIP candidate survives S1. |
| `ssh-key` | 0.6.7, Apache-2.0 OR MIT | Deferred to W2.7 by coordinator ruling. It is absent from every manifest; W2.7 must resolve the applicability of RUSTSEC-2023-0071 before adoption. |
| `typescript` | 7.0.2 | External instrument, deferred to the S1/S6 lock entries that record the selected platform binary. |
| `@typescript/typescript6` | 6.0.2 | S1 compatibility candidate only. |
| `typescript` 5.x | 5.9.3 | Recorded only if a 5.x instrument actually runs. |
| `@sourcegraph/scip-typescript` | 0.4.0 | S1 candidate only; not a Cargo or workspace dependency. |
| `knip` | 6.36.0 | Evidence instrument; defer until an adapter adopts it. |
| `fallow` | 3.27.0 | W2.4 evidence instrument; defer to its instrument-lock adoption. |
| `dependency-cruiser` | 18.3.1 | Optional census importer; no current Cargo dependency. |
| `@stryker-mutator/core` | 10.0.0 | Deferred mutation instrument, with report schemas 3.8.4, until its adapter task. |
| `vitest` | 5.0.1 | W2.4 evidence instrument; defer to its instrument-lock adoption. |
| `jest` | 30.5.1 | Optional evidence adapter; not needed by the current plan slice. |
| TypeSafe System One SDKs | 0.6.0 | Optional hosted judgment backend; the initial deterministic core and offline path do not adopt it. |
| `rust-analyzer` | unpinned | Possible later Rust-reference instrument; not required by M0-M2. |

## Runtime identities observed

The recheck ran with rustc 1.98.1, cargo 1.98.1, cargo-deny 0.20.2, git
2.47.3, OpenSSH 10.0p2, and Node 26.9.0. Cargo metadata confirms the exact
workspace package identities and selected features in `Cargo.lock`. These are
build-host observations, not receipt claims for future executions; runtime
receipts must record the tools and binaries that actually ran.
