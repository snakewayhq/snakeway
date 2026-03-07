use crate::conf::types::StructuredLoggingDeviceSpec;
use crate::device::builtin::structured_logging::{LogEvent, LogLevel, LogPhase};
use crate::enrichment::identity_field::IdentityField;
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

    pub events: Option<Vec<LogEvent>>,

    pub phases: Option<Vec<LogPhase>>,

    pub otel_metrics: bool,
    pub otel_endpoint: Option<String>,
    pub otel_service_name: Option<String>,
}

impl From<StructuredLoggingDeviceSpec> for StructuredLoggingDeviceConfig {
    fn from(spec: StructuredLoggingDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            level: spec.level,
            include_headers: spec.include_headers,
            allowed_headers: spec.allowed_headers,
            redacted_headers: spec.redacted_headers,
            include_identity: spec.include_identity,
            identity_fields: spec.identity_fields,
            events: spec.events,
            phases: spec.phases,
            otel_metrics: spec.otel_metrics,
            otel_endpoint: spec.otel_endpoint,
            otel_service_name: spec.otel_service_name,
        }
    }
}
