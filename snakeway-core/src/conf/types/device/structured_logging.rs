use crate::device::builtin::structured_logging::{IdentityField, LogEvent, LogLevel, LogPhase};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    #[serde(default = "default_level")]
    pub level: LogLevel,

    /// Headers are excluded by default.
    #[serde(default)]
    pub include_headers: bool,

    /// Allowlist of headers to include.
    /// If empty, all headers are eligible (subject to redaction).
    #[serde(default)]
    pub allowed_headers: Vec<String>,

    /// Headers to redact (case-insensitive).
    #[serde(default)]
    pub redact_headers: Vec<String>,

    /// Identity logging.
    #[serde(default)]
    pub include_identity: bool,

    /// Identity fields to include in the request context (and possibly log).
    #[serde(default = "default_identity_fields")]
    pub identity_fields: Vec<IdentityField>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<LogEvent>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<LogPhase>>,
}

fn default_identity_fields() -> Vec<IdentityField> {
    vec![IdentityField::Country, IdentityField::Device]
}

fn default_level() -> LogLevel {
    LogLevel::Info
}
