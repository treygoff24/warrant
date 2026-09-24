//! Pre-declared framework-entrypoint extension point for W1.8.

use warrant_core::nouns::InventoryDocument;
use warrant_model::IntegrationReport;

use crate::{Reader, Result, Unit};

pub(crate) fn augment(
    _unit: &Unit,
    _inventory: &InventoryDocument,
    _read: Reader<'_>,
    _report: &mut IntegrationReport,
) -> Result<()> {
    Ok(())
}
