use schemars::{Schema, schema_for};

use crate::nouns::ErrorDocument;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(ErrorDocument))
}
