//! Owns policy types, compilation, canonical digests, and structural lint; it must not know snapshots.

mod compiler;
mod types;

pub use compiler::{PolicyError, PolicySource, claim_capabilities, compile, compile_at, lint_at};
pub use types::*;

#[cfg(test)]
mod tests;
