//! Pre-declared parity extension point for W1.4.

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
