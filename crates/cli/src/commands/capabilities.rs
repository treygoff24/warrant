use clap::Args as ClapArgs;
use serde::Serialize;

use crate::{cli::Format, error::CommandError, output};

const IMPLEMENTED: &[&str] = &["snapshot", "inventory", "schema", "capabilities"];
const STUBS: &[&str] = &[
    "model",
    "query",
    "context",
    "propose",
    "check",
    "gate",
    "explain",
    "verify",
    "policy",
    "rule",
    "attest",
    "evidence",
    "instrument",
    "census",
    "map",
    "serve",
    "hook",
    "selftest",
    "self-qualify",
    "judgment",
];

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Maximum number of command records returned.
    #[arg(long, default_value_t = 100, value_parser = parse_limit)]
    limit: usize,
    /// Zero-based cursor returned by a previous invocation.
    #[arg(long, default_value_t = 0)]
    cursor: usize,
}

fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| error.to_string())
        .and_then(|limit| {
            (limit > 0)
                .then_some(limit)
                .ok_or_else(|| "limit must be positive".into())
        })
}

#[derive(Serialize)]
struct Capabilities<'a> {
    schema_version: &'static str,
    implemented: Vec<&'a str>,
    stubs: Vec<&'a str>,
    adapters: Vec<&'a str>,
    truncated: bool,
    total: usize,
    next_cursor: Option<usize>,
}

pub fn run(args: Args, format: Option<Format>) -> crate::error::Result<()> {
    let all: Vec<_> = IMPLEMENTED.iter().chain(STUBS).copied().collect();
    if args.cursor > all.len() {
        return Err(CommandError::evaluation(
            "invalid-cursor",
            format!("cursor {} exceeds total {}", args.cursor, all.len()),
            Some("omit --cursor to start from the beginning".into()),
        ));
    }
    let end = args.cursor.saturating_add(args.limit).min(all.len());
    let selected = &all[args.cursor..end];
    let capabilities = Capabilities {
        schema_version: "warrant.capabilities/1",
        implemented: selected
            .iter()
            .copied()
            .filter(|name| IMPLEMENTED.contains(name))
            .collect(),
        stubs: selected
            .iter()
            .copied()
            .filter(|name| STUBS.contains(name))
            .collect(),
        adapters: Vec::new(),
        truncated: end < all.len(),
        total: all.len(),
        next_cursor: (end < all.len()).then_some(end),
    };
    output::document(&capabilities, format)
}
