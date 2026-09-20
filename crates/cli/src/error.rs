use warrant_core::nouns::ErrorDocument;

#[derive(Debug)]
pub struct CommandError {
    pub document: ErrorDocument,
    pub exit: i32,
}

pub type Result<T> = std::result::Result<T, Box<CommandError>>;

impl CommandError {
    pub fn evaluation(code: &str, reason: impl Into<String>, next: Option<String>) -> Box<Self> {
        Box::new(Self::new(2, code, reason, next))
    }

    pub fn internal(reason: impl Into<String>) -> Box<Self> {
        Box::new(Self::new(3, "internal", reason, None))
    }

    pub fn cancelled(signal: i32) -> Box<Self> {
        Box::new(Self::new(
            128 + signal,
            "cancelled",
            format!("cancelled by signal {signal}"),
            None,
        ))
    }

    fn new(
        exit: i32,
        code: &str,
        reason: impl Into<String>,
        next_diagnostic: Option<String>,
    ) -> Self {
        Self {
            document: ErrorDocument {
                schema_version: "warrant.error/1".into(),
                code: code.into(),
                reason: reason.into(),
                locations: Vec::new(),
                next_diagnostic,
            },
            exit,
        }
    }
}

pub fn fail(error: CommandError) -> ! {
    if serde_json::to_writer(std::io::stderr().lock(), &error.document).is_ok() {
        eprintln!();
    }
    std::process::exit(error.exit)
}
