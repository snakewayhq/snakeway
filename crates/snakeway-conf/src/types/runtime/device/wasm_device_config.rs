use crate::types::WasmDeviceSpec;
use confval::provenance::{Lower, Report, Validate};
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

impl Lower<WasmDeviceSpec> for WasmDeviceConfig
where
    WasmDeviceSpec: Validate,
{
    fn lower(spec: &WasmDeviceSpec, _report: &mut Report) -> Option<Self> {
        Some(Self {
            enable: spec.enable.value,
            path: spec.path.value.clone(),
            config: spec.config.clone(),
        })
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
        let config = WasmDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert!(config.enable);
        assert_eq!(config.path, PathBuf::from("/opt/modules/filter.wasm"));
        assert_eq!(config.config, Some(hcl::Value::from("test-config")));
    }
}
