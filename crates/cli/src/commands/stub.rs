use clap::Args as ClapArgs;

use crate::error::CommandError;

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments: Vec<String>,
}

pub fn run(command: &str, args: Args) -> crate::error::Result<()> {
    let _ = args.arguments;
    Err(CommandError::evaluation(
        "not-implemented",
        format!("`warrant {command}` is not implemented"),
        Some("run `warrant capabilities` to list implemented commands".into()),
    ))
}
