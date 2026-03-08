use crate::conf::types::Origin;
use crate::execution::device::builtin::structured_logging::{
    IdentityField, LogEvent, LogLevel, LogPhase,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StructuredLoggingDeviceSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,

    pub(crate) enable: bool,

    pub(crate) level: LogLevel,

    /// Headers are excluded by default.
    pub(crate) include_headers: bool,

    /// Allowlist of headers to include.
    /// If empty, all headers are eligible (subject to redaction).
    pub(crate) allowed_headers: Vec<String>,

    /// Headers to redact (case-insensitive).
    pub(crate) redacted_headers: Vec<String>,

    /// Identity logging.
    pub(crate) include_identity: bool,

    /// Identity fields to include in the request context (and possibly log).
    pub(crate) identity_fields: Vec<IdentityField>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) events: Option<Vec<LogEvent>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) phases: Option<Vec<LogPhase>>,
}
