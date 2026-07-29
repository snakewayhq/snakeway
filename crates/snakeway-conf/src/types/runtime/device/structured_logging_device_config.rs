use crate::types::{IdentityField, LogEvent, LogLevel, LogPhase, StructuredLoggingDeviceSpec};
use confval::prelude::{Located, Lower, Report, Validate, ValidateNested, narrow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceConfig {
    pub enable: bool,
    pub level: LogLevel,
    pub include_headers: bool,
    pub allowed_headers: HashSet<String>,
    pub redacted_headers: HashSet<String>,
    pub include_identity: bool,
    pub identity_fields: Vec<IdentityField>,
    pub events: Option<Vec<LogEvent>>,
    pub phases: Option<Vec<LogPhase>>,
}

impl Lower<StructuredLoggingDeviceSpec> for StructuredLoggingDeviceConfig
where
    StructuredLoggingDeviceSpec: Validate + ValidateNested,
{
    fn lower(spec: &StructuredLoggingDeviceSpec, report: &mut Report) -> Option<Self> {
        fn keywords<T: for<'a> TryFrom<&'a str>>(
            values: &[Located<String>],
            report: &mut Report,
            ok: &mut bool,
        ) -> Vec<T> {
            values
                .iter()
                .filter_map(|value| {
                    let parsed = narrow::keyword(value, report);
                    if parsed.is_none() {
                        *ok = false;
                    }
                    parsed
                })
                .collect()
        }

        let mut ok = true;

        let level = narrow::keyword::<LogLevel>(&spec.level, report);
        if level.is_none() {
            ok = false;
        }

        let config = Self {
            enable: spec.enable.value,
            level: level.unwrap_or_default(),
            include_headers: spec.include_headers.value,
            allowed_headers: spec
                .allowed_headers
                .iter()
                .map(|h| h.value.to_lowercase())
                .collect(),
            redacted_headers: spec
                .redacted_headers
                .iter()
                .map(|h| h.value.to_lowercase())
                .collect(),
            include_identity: spec.include_identity.value,
            identity_fields: keywords(&spec.identity_fields, report, &mut ok),
            events: spec
                .events
                .as_ref()
                .map(|e| keywords(&e.value, report, &mut ok)),
            phases: spec
                .phases
                .as_ref()
                .map(|p| keywords(&p.value, report, &mut ok)),
        };
        ok.then_some(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = StructuredLoggingDeviceSpec {
            enable: Located::detached(true),
            level: Located::detached("info".to_string()),
            include_headers: Located::detached(true),
            allowed_headers: vec![Located::detached("X-Request-Id".to_string())],
            redacted_headers: vec![Located::detached("Authorization".to_string())],
            include_identity: Located::detached(true),
            identity_fields: vec![Located::detached("client_ip".to_string())],
            events: Some(Located::detached(vec![Located::detached(
                "before_proxy".to_string(),
            )])),
            phases: Some(Located::detached(vec![Located::detached(
                "request".to_string(),
            )])),
        };

        // Act
        let config = StructuredLoggingDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert!(config.enable);
        assert!(matches!(config.level, LogLevel::Info));
        assert!(config.allowed_headers.contains("x-request-id"));
        assert!(config.redacted_headers.contains("authorization"));
        assert_eq!(config.identity_fields, vec![IdentityField::ClientIp]);
        assert_eq!(config.events, Some(vec![LogEvent::BeforeProxy]));
        assert_eq!(config.phases, Some(vec![LogPhase::Request]));
    }

    #[test]
    fn unknown_level_fails() {
        // Arrange
        let spec = StructuredLoggingDeviceSpec {
            level: Located::detached("loud".to_string()),
            ..Default::default()
        };
        let mut report = Report::new();

        // Act
        let result = StructuredLoggingDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown keyword: loud")
        );
    }
}
