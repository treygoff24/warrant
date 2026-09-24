/// Whether `warrant capabilities` reports this command as implemented. The task that
/// implements the command flips this constant in the module it owns.
pub const IMPLEMENTED: bool = false;

pub type Args = super::stub::Args;

pub fn run(args: Args) -> crate::error::Result<()> {
    super::stub::run("attest", args)
}
