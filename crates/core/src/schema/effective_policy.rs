use schemars::{Schema, schema_for};

use crate::policy::EffectivePolicy;

pub(super) fn schema() -> Result<Schema, super::SchemaError> {
    Ok(schema_for!(EffectivePolicy))
}
