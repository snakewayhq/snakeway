use crate::types::WasmDeviceSpec;
use confval::prelude::{Lower, Report, Validate, narrow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WasmDeviceFailPolicy {
    #[default]
    Open,
    Closed,
}

impl TryFrom<&str> for WasmDeviceFailPolicy {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, Self::Error> {
        match keyword {
            "open" => Ok(WasmDeviceFailPolicy::Open),
            "closed" => Ok(WasmDeviceFailPolicy::Closed),
            other => Err(format!("unknown fail_policy: {other}")),
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct WasmDeviceConfig {
    pub name: String,

    pub enable: bool,

    pub path: PathBuf,

    pub fail_policy: WasmDeviceFailPolicy,

    pub timeout_ms: u64,

    pub body_buffer_max: u64,

    pub config: HashMap<String, String>,
}

impl Lower<WasmDeviceSpec> for WasmDeviceConfig
where
    WasmDeviceSpec: Validate,
{
    fn lower(spec: &WasmDeviceSpec, report: &mut Report) -> Option<Self> {
        let fail_policy = match WasmDeviceFailPolicy::try_from(spec.fail_policy.value.as_str()) {
            Ok(policy) => policy,
            Err(message) => {
                report.error(message).at(spec.fail_policy.span).emit();
                return None;
            }
        };

        let timeout_ms = narrow::i64_to_u64(&spec.timeout_ms, report)?;
        let body_buffer_max = narrow::i64_to_u64(&spec.body_buffer_max, report)?;

        Some(Self {
            name: spec.name.value.clone(),
            enable: spec.enable.value,
            path: spec.path.value.clone(),
            fail_policy,
            timeout_ms,
            body_buffer_max,
            config: spec.config.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::{Located, Report};

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = WasmDeviceSpec {
            name: Located::detached("auth-gateway".to_string()),
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/opt/modules/filter.wasm")),
            fail_policy: Located::detached("open".to_string()),
            timeout_ms: Located::detached(10),
            body_buffer_max: Located::detached(65536),
            config: HashMap::from([("key".to_string(), "value".to_string())]),
        };

        // Act
        let config = WasmDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.name, "auth-gateway");
        assert!(config.enable);
        assert_eq!(config.path, PathBuf::from("/opt/modules/filter.wasm"));
        assert_eq!(config.fail_policy, WasmDeviceFailPolicy::Open);
        assert_eq!(config.timeout_ms, 10);
        assert_eq!(config.body_buffer_max, 65536);
        assert_eq!(config.config.get("key").unwrap(), "value");
    }

    #[test]
    fn from_spec_closed_policy() {
        // Arrange
        let spec = WasmDeviceSpec {
            fail_policy: Located::detached("closed".to_string()),
            ..Default::default()
        };

        // Act
        let config = WasmDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.fail_policy, WasmDeviceFailPolicy::Closed);
    }

    #[test]
    fn from_spec_default_timeout() {
        // Arrange
        let spec = WasmDeviceSpec::default();

        // Act
        let config = WasmDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.timeout_ms, 5);
    }

    #[test]
    fn unknown_fail_policy_fails_lowering() {
        // Arrange
        let spec = WasmDeviceSpec {
            fail_policy: Located::detached("maybe".to_string()),
            ..Default::default()
        };
        let mut report = Report::new();

        // Act
        let result = WasmDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown fail_policy: maybe")
        );
    }
}
