use crate::device::builtin::structured_logging::{IdentityField, LogEvent, LogLevel, LogPhase};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceConfig {
    pub enable: bool,

    pub level: LogLevel,

    /// Headers are excluded by default.
    pub include_headers: bool,

    /// Allowlist of headers to include.
    /// If empty, all headers are eligible (subject to redaction).
    pub allowed_headers: Vec<String>,

    /// Headers to redact (case-insensitive).
    pub redacted_headers: Vec<String>,

    /// Identity logging.
    pub include_identity: bool,

    /// Identity fields to include in the request context (and possibly log).
    pub identity_fields: Vec<IdentityField>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<LogEvent>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<LogPhase>>,
}
