use clap::{Args as ClapArgs, Subcommand};
use warrant_core::{
    nouns::{CommandRecord, CommandStatus, CommandsDocument},
    schema::DOCUMENTS,
};

use crate::{cli, cli::Format, output};

use super::page;

const IMPLEMENTED: &[&str] = &["snapshot", "inventory", "schema", "capabilities"];

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Maximum number of command records returned.
    #[arg(long, default_value_t = 100, value_parser = page::parse_limit)]
    limit: usize,
    /// Zero-based cursor returned by a previous invocation.
    #[arg(long, default_value_t = 0)]
    cursor: usize,
}

pub fn run(args: Args, format: Option<Format>) -> crate::error::Result<()> {
    // Clap retains enum declaration order in its generated subcommands.
    let registered =
        <cli::Command as Subcommand>::augment_subcommands(clap::Command::new("warrant"));
    let commands: Vec<_> = registered
        .get_subcommands()
        .map(|command| {
            let name = command.get_name();
            CommandRecord {
                name: name.to_owned(),
                status: if IMPLEMENTED.contains(&name) {
                    CommandStatus::Implemented
                } else {
                    CommandStatus::Stub
                },
            }
        })
        .collect();
    let page = page::bounds(commands.len(), args.limit, args.cursor)?;
    let document = CommandsDocument {
        schema_version: "warrant.commands/1".into(),
        commands: commands[page.range].to_vec(),
        implemented: IMPLEMENTED.iter().map(|name| (*name).into()).collect(),
        adapters: Vec::new(),
        schemas: DOCUMENTS
            .iter()
            .filter(|document| document.implemented())
            .map(|document| document.name.into())
            .collect(),
        truncated: page.truncated,
        total: page.total,
        next_cursor: page.next_cursor,
    };
    output::document(&document, format)
}
