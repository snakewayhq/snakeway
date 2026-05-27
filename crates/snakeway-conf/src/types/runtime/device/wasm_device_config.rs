use crate::types::WasmDeviceSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(o2o, Default, Debug, Clone, Deserialize, Serialize)]
#[from_owned(WasmDeviceSpec)]
pub struct WasmDeviceConfig {
    pub enable: bool,

    /// The location of the WASM module.
    pub path: PathBuf,

    /// Device-specific configuration blob
    pub config: Option<hcl::Value>,
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
