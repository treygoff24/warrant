pub type Args = super::stub::Args;

pub fn run(args: Args) -> crate::error::Result<()> {
    super::stub::run("explain", args)
}
