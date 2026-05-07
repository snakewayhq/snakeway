use crate::types::WasmDeviceSpec;

use super::WasmDeviceConfig;

impl From<WasmDeviceSpec> for WasmDeviceConfig {
    fn from(spec: WasmDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            path: spec.path,
            config: spec.config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OriginDeprecated;
    use std::path::PathBuf;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = WasmDeviceSpec {
            origin: OriginDeprecated::default(),
            enable: true,
            path: PathBuf::from("/opt/modules/filter.wasm"),
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
