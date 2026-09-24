/// Whether `warrant capabilities` reports this command as implemented. The task that
/// implements the command flips this constant in the module it owns.
pub const IMPLEMENTED: bool = true;

use clap::Args as ClapArgs;

use crate::{cli::Format, error::CommandError, output};

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Stable schema name, for example warrant.snapshot.
    pub name: String,
}

pub fn run(args: Args, _format: Option<Format>) -> crate::error::Result<()> {
    let schema = warrant_core::schema::generate(&args.name).map_err(|error| {
        CommandError::evaluation(
            "not-implemented",
            error.to_string(),
            Some(
                "run `warrant capabilities` and read `schemas` for the implemented schema names"
                    .into(),
            ),
        )
    })?;
    let bytes = warrant_core::schema::canonical_bytes(&schema)
        .map_err(|error| CommandError::internal(format!("could not encode schema: {error}")))?;
    output::bytes(&bytes)
}
