use schemars::{Schema, schema_for};

use crate::manifest::WarrantManifest;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(WarrantManifest))
}
