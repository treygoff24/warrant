use std::io::{self, IsTerminal, Write};

use serde::Serialize;

use crate::{cli::Format, error::CommandError};

pub fn selected(explicit: Option<Format>) -> Format {
    explicit.unwrap_or_else(|| {
        if io::stdout().is_terminal() {
            Format::Human
        } else {
            Format::Json
        }
    })
}

pub fn document(value: &impl Serialize, format: Option<Format>) -> crate::error::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    match selected(format) {
        Format::Json => serde_json::to_writer(&mut writer, value),
        Format::Human => serde_json::to_writer_pretty(&mut writer, value),
    }
    .map_err(|error| CommandError::internal(format!("could not write output: {error}")))?;
    writeln!(writer)
        .map_err(|error| CommandError::internal(format!("could not write output: {error}")))
}

pub fn bytes(bytes: &[u8]) -> crate::error::Result<()> {
    io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|error| CommandError::internal(format!("could not write output: {error}")))
}
