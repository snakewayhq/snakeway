use crate::types::{
    IdentityFieldSpec, LogEventSpec, LogLevelSpec, LogPhaseSpec, StructuredLoggingDeviceSpec,
};

use super::{
    IdentityFieldConfig, LogEventConfig, LogLevelConfig, LogPhaseConfig,
    StructuredLoggingDeviceConfig,
};

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

impl From<LogPhaseSpec> for LogPhaseConfig {
    fn from(phase: LogPhaseSpec) -> Self {
        match phase {
            LogPhaseSpec::Request => LogPhaseConfig::Request,
            LogPhaseSpec::Response => LogPhaseConfig::Response,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::specification::HclOrigin;
    use std::collections::HashSet;

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
