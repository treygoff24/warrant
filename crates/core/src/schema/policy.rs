use schemars::{Schema, schema_for};

use crate::policy::PolicyDocument;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(PolicyDocument))
}
