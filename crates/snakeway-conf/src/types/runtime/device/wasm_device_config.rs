use crate::types::{WasmDeviceFailPolicy, WasmDeviceSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct WasmDeviceConfig {
    pub name: String,

    pub enable: bool,

    /// The location of the WASM module.
    pub path: PathBuf,

    pub fail_policy: WasmDeviceFailPolicy,

    pub timeout_ms: u64,

    pub body_buffer_max: u64,

    pub config: HashMap<String, String>,
}

impl From<WasmDeviceSpec> for WasmDeviceConfig {
    fn from(spec: WasmDeviceSpec) -> Self {
        Self {
            name: spec.name,
            enable: spec.enable,
            path: spec.path,
            fail_policy: spec.fail_policy,
            timeout_ms: spec.timeout_ms as u64,
            body_buffer_max: spec.body_buffer_max as u64,
            config: spec.config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HclOrigin;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = WasmDeviceSpec {
            origin: HclOrigin::default(),
            name: "auth-gateway".to_string(),
            enable: true,
            path: PathBuf::from("/opt/modules/filter.wasm"),
            fail_policy: WasmDeviceFailPolicy::Open,
            timeout_ms: 10,
            body_buffer_max: 65536,
            config: HashMap::from([("key".to_string(), "value".to_string())]),
        };

        // Act
        let config: WasmDeviceConfig = spec.into();

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
            fail_policy: WasmDeviceFailPolicy::Closed,
            ..Default::default()
        };

        // Act
        let config: WasmDeviceConfig = spec.into();

        // Assert
        assert_eq!(config.fail_policy, WasmDeviceFailPolicy::Closed);
    }

    #[test]
    fn from_spec_default_timeout() {
        // Arrange
        let spec = WasmDeviceSpec::default();

        // Act
        let config: WasmDeviceConfig = spec.into();

        // Assert
        assert_eq!(config.timeout_ms, 5);
    }
}
