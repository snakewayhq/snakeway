use crate::types::{IdentityFieldSpec, LogEventSpec, LogLevelSpec, LogPhaseSpec};
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceConfig {
    pub enable: bool,

    pub level: LogLevelConfig,

    /// Headers are excluded by default.
    pub include_headers: bool,

    /// Allowlist of headers to include.
    /// If empty, all headers are eligible (subject to redaction).
    pub allowed_headers: HashSet<String>,

    /// Headers to redact (case-insensitive).
    pub redacted_headers: HashSet<String>,

    /// Identity logging.
    pub include_identity: bool,

    /// Identity fields to include in the request context (and possibly log).
    pub identity_fields: Vec<IdentityFieldConfig>,

    pub events: Option<Vec<LogEventConfig>>,

    pub phases: Option<Vec<LogPhaseConfig>>,
}

#[derive(o2o, Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[from_owned(LogLevelSpec)]
#[serde(rename_all = "lowercase")]
pub enum LogLevelConfig {
    Trace,
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

#[derive(o2o, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[from_owned(LogEventSpec)]
#[serde(rename_all = "snake_case")]
pub enum LogEventConfig {
    Request,
    BeforeProxy,
    AfterProxy,
    Response,
}

#[derive(o2o, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[from_owned(LogPhaseSpec)]
#[serde(rename_all = "lowercase")]
pub enum LogPhaseConfig {
    Request,
    Response,
}

#[derive(o2o, Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[from_owned(IdentityFieldSpec)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_from_spec() {
        // Arrange / Act / Assert
        assert!(matches!(
            LogLevelConfig::from(LogLevelSpec::Trace),
            LogLevelConfig::Trace
        ));
        assert!(matches!(
            LogLevelConfig::from(LogLevelSpec::Debug),
            LogLevelConfig::Debug
        ));
        assert!(matches!(
            LogLevelConfig::from(LogLevelSpec::Info),
            LogLevelConfig::Info
        ));
        assert!(matches!(
            LogLevelConfig::from(LogLevelSpec::Warn),
            LogLevelConfig::Warn
        ));
        assert!(matches!(
            LogLevelConfig::from(LogLevelSpec::Error),
            LogLevelConfig::Error
        ));
    }
}
