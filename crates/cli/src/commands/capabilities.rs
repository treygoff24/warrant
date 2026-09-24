/// Whether `warrant capabilities` reports this command as implemented. The task that
/// implements the command flips this constant in the module it owns.
pub const IMPLEMENTED: bool = true;

use clap::{Args as ClapArgs, Subcommand};
use warrant_core::{
    nouns::{CommandRecord, CommandStatus, CommandsDocument},
    schema::DOCUMENTS,
};

use crate::{cli, cli::Format, output};

use super::page;

/// Each command module declares its own status; `self-qualify` shares the selftest module.
fn implemented(name: &str) -> bool {
    match name {
        "snapshot" => super::snapshot::IMPLEMENTED,
        "inventory" => super::inventory::IMPLEMENTED,
        "model" => super::model::IMPLEMENTED,
        "query" => super::query::IMPLEMENTED,
        "context" => super::context::IMPLEMENTED,
        "propose" => super::propose::IMPLEMENTED,
        "check" => super::check::IMPLEMENTED,
        "gate" => super::gate::IMPLEMENTED,
        "explain" => super::explain::IMPLEMENTED,
        "verify" => super::verify::IMPLEMENTED,
        "policy" => super::policy::IMPLEMENTED,
        "rule" => super::rule::IMPLEMENTED,
        "attest" => super::attest::IMPLEMENTED,
        "evidence" => super::evidence::IMPLEMENTED,
        "instrument" => super::instrument::IMPLEMENTED,
        "census" => super::census::IMPLEMENTED,
        "map" => super::map::IMPLEMENTED,
        "serve" => super::serve::IMPLEMENTED,
        "hook" => super::hook::IMPLEMENTED,
        "selftest" | "self-qualify" => super::selftest::IMPLEMENTED,
        "judgment" => super::judgment::IMPLEMENTED,
        "schema" => super::schema::IMPLEMENTED,
        "capabilities" => IMPLEMENTED,
        _ => false,
    }
}

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
                status: if implemented(name) {
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
        implemented: commands
            .iter()
            .filter(|record| record.status == CommandStatus::Implemented)
            .map(|record| record.name.clone())
            .collect(),
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
