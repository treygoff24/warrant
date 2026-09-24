//! Pre-declared extension point for policy facts, implemented by W1.8.

use crate::{ModelBuilder, Result, UnitRow};

/// Add policy-declared facts for one analyzed unit.
///
/// W1.8 replaces this empty implementation without changing the build path.
pub(crate) fn write_for_unit(_builder: &mut ModelBuilder, _unit: &UnitRow) -> Result<()> {
    Ok(())
}
