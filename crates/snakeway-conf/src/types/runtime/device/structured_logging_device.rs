use crate::types::{
    IdentityFieldSpec, LogEventSpec, LogLevelSpec, LogPhaseSpec, StructuredLoggingDeviceSpec,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceConfig {
    pub enable: bool,

    pub level: LogLevelConfig,

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
    pub identity_fields: Vec<IdentityFieldConfig>,

    pub events: Option<Vec<LogEventConfig>>,

    pub phases: Option<Vec<LogPhaseConfig>>,
}

impl From<StructuredLoggingDeviceSpec> for StructuredLoggingDeviceConfig {
    fn from(spec: StructuredLoggingDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            level: spec.level.into(),
            include_headers: spec.include_headers,
            allowed_headers: spec.allowed_headers,
            redacted_headers: spec.redacted_headers,
            include_identity: spec.include_identity,
            identity_fields: spec.identity_fields.into_iter().map(|f| f.into()).collect(),
            events: spec
                .events
                .map(|e| e.into_iter().map(|e| e.into()).collect()),
            phases: spec
                .phases
                .map(|p| p.into_iter().map(|p| p.into()).collect()),
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevelConfig {
    Trace,
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

impl From<LogLevelSpec> for LogLevelConfig {
    fn from(level: LogLevelSpec) -> Self {
        match level {
            LogLevelSpec::Trace => LogLevelConfig::Trace,
            LogLevelSpec::Debug => LogLevelConfig::Debug,
            LogLevelSpec::Info => LogLevelConfig::Info,
            LogLevelSpec::Warn => LogLevelConfig::Warn,
            LogLevelSpec::Error => LogLevelConfig::Error,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogEventConfig {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

impl From<LogEventSpec> for LogEventConfig {
    fn from(event: LogEventSpec) -> Self {
        match event {
            LogEventSpec::Request => LogEventConfig::Request,
            LogEventSpec::BeforeProxy => LogEventConfig::BeforeProxy,
            LogEventSpec::AfterProxy => LogEventConfig::AfterProxy,
            LogEventSpec::Response => LogEventConfig::Response,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogPhaseConfig {
    Request,
    Response,
}

impl From<LogPhaseSpec> for LogPhaseConfig {
    fn from(phase: LogPhaseSpec) -> Self {
        match phase {
            LogPhaseSpec::Request => LogPhaseConfig::Request,
            LogPhaseSpec::Response => LogPhaseConfig::Response,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityFieldConfig {
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

impl From<IdentityFieldSpec> for IdentityFieldConfig {
    fn from(value: IdentityFieldSpec) -> Self {
        match value {
            IdentityFieldSpec::ClientIp => IdentityFieldConfig::ClientIp,
            IdentityFieldSpec::ProxyChain => IdentityFieldConfig::ProxyChain,
            IdentityFieldSpec::Forwarded => IdentityFieldConfig::Forwarded,
            IdentityFieldSpec::Trusted => IdentityFieldConfig::Trusted,

            IdentityFieldSpec::Asn => IdentityFieldConfig::Asn,
            IdentityFieldSpec::Aso => IdentityFieldConfig::Aso,
            IdentityFieldSpec::Country => IdentityFieldConfig::Country,
            IdentityFieldSpec::Region => IdentityFieldConfig::Region,
            IdentityFieldSpec::ConnectionType => IdentityFieldConfig::ConnectionType,

            IdentityFieldSpec::Bot => IdentityFieldConfig::Bot,
            IdentityFieldSpec::Device => IdentityFieldConfig::Device,
        }
    }
}
