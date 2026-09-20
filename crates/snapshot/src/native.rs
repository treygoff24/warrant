//! S3 owns this module. Unsupported keeps the portable Git path authoritative.
use crate::SnapshotError;
use std::path::Path;
use warrant_core::nouns::SnapshotKind;

/// Return a native tree id when supported, otherwise use the temporary index.
pub fn capture(_repo: &Path, _kind: SnapshotKind) -> Result<Option<String>, SnapshotError> {
    Ok(None)
}
