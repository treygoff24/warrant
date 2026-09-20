mod cache;
mod cancel;
mod cli;
mod commands;
mod error;
mod output;

use clap::{CommandFactory, Parser, error::ErrorKind};
use cli::Cli;

fn main() {
    cancel::install().unwrap_or_else(|reason| {
        error::fail(*error::CommandError::internal(reason));
    });

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(parse_error)
            if matches!(
                parse_error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{parse_error}");
            return;
        }
        Err(parse_error) => {
            let usage = Cli::command().render_usage().to_string();
            error::fail(*error::CommandError::evaluation(
                "invalid-invocation",
                parse_error.to_string().trim().to_owned(),
                Some(usage),
            ));
        }
    };

    if let Err(command_error) = commands::run(cli.command, cli.format) {
        error::fail(*command_error);
    }
}
