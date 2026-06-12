use crate::types::WasmDeviceSpec;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct WasmDeviceConfig {
    pub enable: bool,

    /// The location of the WASM module.
    pub path: PathBuf,

    /// Device-specific configuration blob
    pub config: Option<hcl::Value>,
}

impl From<WasmDeviceSpec> for WasmDeviceConfig {
    fn from(spec: WasmDeviceSpec) -> Self {
        Self {
            enable: spec.enable.value,
            path: spec.path.value,
            config: spec.config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        use confval::provenance::Located;
        let spec = WasmDeviceSpec {
            enable: Located::detached(true),
            path: Located::detached(PathBuf::from("/opt/modules/filter.wasm")),
            config: Some(hcl::Value::from("test-config")),
        };

        // Act
        let config: WasmDeviceConfig = spec.into();

        // Assert
        assert!(config.enable);
        assert_eq!(config.path, PathBuf::from("/opt/modules/filter.wasm"));
        assert_eq!(config.config, Some(hcl::Value::from("test-config")));
    }
}
