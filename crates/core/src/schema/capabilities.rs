use schemars::{Schema, schema_for};

use crate::nouns::CapabilityReport;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(CapabilityReport))
}
