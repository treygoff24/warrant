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
    let result = match selected(format) {
        Format::Json => serde_json::to_writer(&mut writer, value),
        Format::Human => serde_json::to_writer_pretty(&mut writer, value),
    };
    if let Err(error) = result {
        return if error.io_error_kind() == Some(io::ErrorKind::BrokenPipe) {
            Ok(())
        } else {
            Err(CommandError::internal(format!(
                "could not write output: {error}"
            )))
        };
    }
    write_result(writeln!(writer).and_then(|()| writer.flush()))
}

/// Exit 1 after a finding-bearing document is on stdout: the document is the result and
/// the exit code is its verdict (spec 10.2, blocked), so nothing goes to stderr.
pub fn exit_blocked() -> ! {
    std::process::exit(1)
}

pub fn bytes(bytes: &[u8]) -> crate::error::Result<()> {
    let mut writer = io::stdout().lock();
    write_result(writer.write_all(bytes).and_then(|()| writer.flush()))
}

fn write_result(result: io::Result<()>) -> crate::error::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(CommandError::internal(format!(
            "could not write output: {error}"
        ))),
    }
}
