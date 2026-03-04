use crate::conf::types::Origin;
use crate::device::builtin::structured_logging::{LogEvent, LogLevel, LogPhase};
use crate::enrichment::identity_field::IdentityField;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

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

    /// Emit OTel spans and metrics for each request/response.
    /// Requires OTEL_EXPORTER_OTLP_ENDPOINT to be set at startup.
    #[serde(default)]
    pub otel_metrics: bool,
}
