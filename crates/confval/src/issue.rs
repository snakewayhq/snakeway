use crate::origin::Origin;
use crate::severity::Severity;

#[derive(Debug, Clone)]
pub struct ValidationIssue<O: Origin> {
    pub severity: Severity,
    pub message: String,
    pub origin: O,
    pub help: Option<String>,
}

impl<O: Origin> ValidationIssue<O> {
    pub fn error(message: impl Into<String>, origin: O) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            origin,
            help: None,
        }
    }

    pub fn error_with_help(message: impl Into<String>, origin: O, help: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            origin,
            help: Some(help.into()),
        }
    }

    pub fn warning(message: impl Into<String>, origin: O) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            origin,
            help: None,
        }
    }

    pub fn warning_with_help(
        message: impl Into<String>,
        origin: O,
        help: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            origin,
            help: Some(help.into()),
        }
    }
}
