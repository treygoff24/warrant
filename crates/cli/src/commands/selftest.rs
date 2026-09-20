pub type Args = super::stub::Args;

pub fn run(args: Args, command: &str) -> crate::error::Result<()> {
    super::stub::run(command, args)
}
