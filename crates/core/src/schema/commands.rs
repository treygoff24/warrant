use schemars::{Schema, schema_for};

use crate::nouns::CommandsDocument;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(CommandsDocument))
}
