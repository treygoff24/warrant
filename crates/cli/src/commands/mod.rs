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
        cli::Command::Model(args) => model::run(args),
        cli::Command::Query(args) => query::run(args),
        cli::Command::Context(args) => context::run(args),
        cli::Command::Propose(args) => propose::run(args),
        cli::Command::Check(args) => check::run(args),
        cli::Command::Gate(args) => gate::run(args),
        cli::Command::Explain(args) => explain::run(args),
        cli::Command::Verify(args) => verify::run(args),
        cli::Command::Policy(args) => policy::run(args),
        cli::Command::Rule(args) => rule::run(args),
        cli::Command::Attest(args) => attest::run(args),
        cli::Command::Evidence(args) => evidence::run(args),
        cli::Command::Instrument(args) => instrument::run(args),
        cli::Command::Census(args) => census::run(args),
        cli::Command::Map(args) => map::run(args),
        cli::Command::Serve(args) => serve::run(args),
        cli::Command::Hook(args) => hook::run(args),
        cli::Command::Selftest(args) => selftest::run(args, "selftest"),
        cli::Command::SelfQualify(args) => selftest::run(args, "self-qualify"),
        cli::Command::Judgment(args) => judgment::run(args),
        cli::Command::Schema(args) => schema::run(args, format),
        cli::Command::Capabilities(args) => capabilities::run(args, format),
    }
}
