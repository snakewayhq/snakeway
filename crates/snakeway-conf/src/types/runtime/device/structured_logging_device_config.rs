use crate::types::{
    IdentityFieldSpec, LogEventSpec, LogLevelSpec, LogPhaseSpec, StructuredLoggingDeviceSpec,
};
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

impl From<StructuredLoggingDeviceSpec> for StructuredLoggingDeviceConfig {
    fn from(spec: StructuredLoggingDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            level: spec.level.into(),
            include_headers: spec.include_headers,
            allowed_headers: spec
                .allowed_headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect(),
            redacted_headers: spec
                .redacted_headers
                .into_iter()
                .map(|h| h.to_lowercase())
                .collect(),
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
    use crate::types::HclOrigin;

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

    #[test]
    fn from_spec_maps_basic_fields() {
        // Arrange
        let spec = StructuredLoggingDeviceSpec {
            origin: HclOrigin::default(),
            enable: true,
            level: LogLevelSpec::Info,
            include_headers: true,
            allowed_headers: vec!["content-type".to_string()],
            redacted_headers: vec!["authorization".to_string()],
            include_identity: true,
            identity_fields: vec![IdentityFieldSpec::ClientIp],
            events: None,
            phases: None,
        };

        // Act
        let config: StructuredLoggingDeviceConfig = spec.into();

        // Assert
        assert!(config.enable);
        assert!(matches!(config.level, LogLevelConfig::Info));
        assert!(config.include_headers);
        assert_eq!(
            config.allowed_headers,
            HashSet::from(["content-type".to_string()])
        );
        assert_eq!(
            config.redacted_headers,
            HashSet::from(["authorization".to_string()])
        );
        assert!(config.include_identity);
        assert_eq!(config.identity_fields.len(), 1);
        assert!(matches!(
            config.identity_fields[0],
            IdentityFieldConfig::ClientIp
        ));
        assert!(config.events.is_none());
        assert!(config.phases.is_none());
    }

    #[test]
    fn from_spec_maps_events_and_phases() {
        // Arrange
        let spec = StructuredLoggingDeviceSpec {
            origin: HclOrigin::default(),
            enable: true,
            level: LogLevelSpec::Info,
            include_headers: false,
            allowed_headers: vec![],
            redacted_headers: vec![],
            include_identity: false,
            identity_fields: vec![],
            events: Some(vec![LogEventSpec::Request, LogEventSpec::Response]),
            phases: Some(vec![LogPhaseSpec::Request, LogPhaseSpec::Response]),
        };

        // Act
        let config: StructuredLoggingDeviceConfig = spec.into();

        // Assert
        let events = config.events.expect("events should be Some");
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], LogEventConfig::Request));
        assert!(matches!(events[1], LogEventConfig::Response));

        let phases = config.phases.expect("phases should be Some");
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], LogPhaseConfig::Request));
        assert!(matches!(phases[1], LogPhaseConfig::Response));
    }
}
