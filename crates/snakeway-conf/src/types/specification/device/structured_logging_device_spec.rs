use crate::types::HclOrigin;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

    pub enable: bool,

    pub level: LogLevelSpec,

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
    pub identity_fields: Vec<IdentityFieldSpec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<LogEventSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<LogPhaseSpec>>,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevelSpec {
    Trace,
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogEventSpec {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogPhaseSpec {
    Request,
    Response,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityFieldSpec {
    ClientIp,
    ProxyChain,
    Forwarded,
    Trusted,

    Asn,
    Aso,
    Country,
    Region,
    ConnectionType,

    Bot,
    Device,
}
