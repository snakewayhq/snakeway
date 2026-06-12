use crate::types::StructuredLoggingDeviceSpec;
use confval::provenance::Located;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredLoggingDeviceConfig {
    pub enable: bool,
    pub level: LogLevelConfig,
    pub include_headers: bool,
    pub allowed_headers: HashSet<String>,
    pub redacted_headers: HashSet<String>,
    pub include_identity: bool,
    pub identity_fields: Vec<IdentityFieldConfig>,
    pub events: Option<Vec<LogEventConfig>>,
    pub phases: Option<Vec<LogPhaseConfig>>,
}

impl TryFrom<StructuredLoggingDeviceSpec> for StructuredLoggingDeviceConfig {
    type Error = String;

    fn try_from(spec: StructuredLoggingDeviceSpec) -> Result<Self, Self::Error> {
        fn keywords<T: for<'a> TryFrom<&'a str, Error = String>>(
            values: Vec<Located<String>>,
        ) -> Result<Vec<T>, String> {
            values
                .into_iter()
                .map(|value| value.value.as_str().try_into())
                .collect()
        }

        Ok(Self {
            enable: spec.enable.value,
            level: spec.level.value.as_str().try_into()?,
            include_headers: spec.include_headers.value,
            allowed_headers: spec
                .allowed_headers
                .into_iter()
                .map(|h| h.value.to_lowercase())
                .collect(),
            redacted_headers: spec
                .redacted_headers
                .into_iter()
                .map(|h| h.value.to_lowercase())
                .collect(),
            include_identity: spec.include_identity.value,
            identity_fields: keywords(spec.identity_fields)?,
            events: spec.events.map(|e| keywords(e.value)).transpose()?,
            phases: spec.phases.map(|p| keywords(p.value)).transpose()?,
        })
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

impl TryFrom<&str> for LogLevelConfig {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, String> {
        match keyword {
            "trace" => Ok(LogLevelConfig::Trace),
            "debug" => Ok(LogLevelConfig::Debug),
            "info" => Ok(LogLevelConfig::Info),
            "warn" => Ok(LogLevelConfig::Warn),
            "error" => Ok(LogLevelConfig::Error),
            other => Err(format!("unknown level: {other}")),
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

impl TryFrom<&str> for LogEventConfig {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, String> {
        match keyword {
            "request" => Ok(LogEventConfig::Request),
            "before_proxy" => Ok(LogEventConfig::BeforeProxy),
            "after_proxy" => Ok(LogEventConfig::AfterProxy),
            "response" => Ok(LogEventConfig::Response),
            other => Err(format!("unknown event: {other}")),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogPhaseConfig {
    Request,
    Response,
}

impl TryFrom<&str> for LogPhaseConfig {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, String> {
        match keyword {
            "request" => Ok(LogPhaseConfig::Request),
            "response" => Ok(LogPhaseConfig::Response),
            other => Err(format!("unknown phase: {other}")),
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

impl TryFrom<&str> for IdentityFieldConfig {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, String> {
        match keyword {
            "client_ip" => Ok(IdentityFieldConfig::ClientIp),
            "proxy_chain" => Ok(IdentityFieldConfig::ProxyChain),
            "forwarded" => Ok(IdentityFieldConfig::Forwarded),
            "trusted" => Ok(IdentityFieldConfig::Trusted),
            "asn" => Ok(IdentityFieldConfig::Asn),
            "aso" => Ok(IdentityFieldConfig::Aso),
            "country" => Ok(IdentityFieldConfig::Country),
            "region" => Ok(IdentityFieldConfig::Region),
            "connection_type" => Ok(IdentityFieldConfig::ConnectionType),
            "bot" => Ok(IdentityFieldConfig::Bot),
            "device" => Ok(IdentityFieldConfig::Device),
            other => Err(format!("unknown identity field: {other}")),
        }
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
        let config = StructuredLoggingDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert!(config.enable);
        assert!(matches!(config.level, LogLevelConfig::Info));
        assert!(config.allowed_headers.contains("x-request-id"));
        assert!(config.redacted_headers.contains("authorization"));
        assert_eq!(config.identity_fields, vec![IdentityFieldConfig::ClientIp]);
        assert_eq!(config.events, Some(vec![LogEventConfig::BeforeProxy]));
        assert_eq!(config.phases, Some(vec![LogPhaseConfig::Request]));
    }

    #[test]
    fn unknown_level_fails() {
        // Arrange
        let spec = StructuredLoggingDeviceSpec {
            level: Located::detached("loud".to_string()),
            ..Default::default()
        };

        // Act
        let result = StructuredLoggingDeviceConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
    }
}
