use crate::cli;

pub mod attest;
pub mod capabilities;
pub mod census;
pub mod check;
pub mod context;
pub mod evidence;
pub mod explain;
pub mod gate;
pub mod hook;
pub mod instrument;
pub mod inventory;
pub mod judgment;
pub mod map;
pub mod model;
mod page;
pub mod policy;
pub mod propose;
pub mod query;
pub mod rule;
pub mod schema;
pub mod selftest;
pub mod serve;
pub mod snapshot;
mod stub;
pub mod verify;

pub fn run(command: cli::Command, format: Option<cli::Format>) -> crate::error::Result<()> {
    match command {
        cli::Command::Snapshot(args) => snapshot::run(args, format),
        cli::Command::Inventory(args) => inventory::run(args, format),
        cli::Command::Model(args) => model::run(args, format),
        cli::Command::Query(args) => query::run(args, format),
        cli::Command::Context(args) => context::run(args, format),
        cli::Command::Propose(args) => propose::run(args, format),
        cli::Command::Check(args) => check::run(args, format),
        cli::Command::Gate(args) => gate::run(args, format),
        cli::Command::Explain(args) => explain::run(args, format),
        cli::Command::Verify(args) => verify::run(args, format),
        cli::Command::Policy(args) => policy::run(args, format),
        cli::Command::Rule(args) => rule::run(args, format),
        cli::Command::Attest(args) => attest::run(args, format),
        cli::Command::Evidence(args) => evidence::run(args, format),
        cli::Command::Instrument(args) => instrument::run(args, format),
        cli::Command::Census(args) => census::run(args, format),
        cli::Command::Map(args) => map::run(args, format),
        cli::Command::Serve(args) => serve::run(args, format),
        cli::Command::Hook(args) => hook::run(args, format),
        cli::Command::Selftest(args) => selftest::run(args, "selftest", format),
        cli::Command::SelfQualify(args) => selftest::run(args, "self-qualify", format),
        cli::Command::Judgment(args) => judgment::run(args, format),
        cli::Command::Schema(args) => schema::run(args, format),
        cli::Command::Capabilities(args) => capabilities::run(args, format),
    }
}
