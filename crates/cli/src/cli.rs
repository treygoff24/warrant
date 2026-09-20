use clap::{Parser, Subcommand, ValueEnum};

use crate::commands;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    Json,
    Human,
}

#[derive(Debug, Parser)]
#[command(
    name = "warrant",
    version,
    about = "Architecture agents can read and cannot cheat",
    subcommand_required = true,
    arg_required_else_help = true,
    color = clap::ColorChoice::Never
)]
pub struct Cli {
    /// Select stable machine JSON or a human rendering of the same data.
    #[arg(long, global = true, value_enum)]
    pub format: Option<Format>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Snapshot(commands::snapshot::Args),
    Inventory(commands::inventory::Args),
    Model(commands::model::Args),
    Query(commands::query::Args),
    Context(commands::context::Args),
    Propose(commands::propose::Args),
    Check(commands::check::Args),
    Gate(commands::gate::Args),
    Explain(commands::explain::Args),
    Verify(commands::verify::Args),
    Policy(commands::policy::Args),
    Rule(commands::rule::Args),
    Attest(commands::attest::Args),
    Evidence(commands::evidence::Args),
    Instrument(commands::instrument::Args),
    Census(commands::census::Args),
    Map(commands::map::Args),
    Serve(commands::serve::Args),
    Hook(commands::hook::Args),
    Selftest(commands::selftest::Args),
    #[command(name = "self-qualify")]
    SelfQualify(commands::selftest::Args),
    Judgment(commands::judgment::Args),
    Schema(commands::schema::Args),
    Capabilities(commands::capabilities::Args),
}
