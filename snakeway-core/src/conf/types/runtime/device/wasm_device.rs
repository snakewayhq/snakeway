use crate::conf::types::WasmDeviceSpec;
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
    fn from(d: WasmDeviceSpec) -> Self {
        Self {
            enable: d.enable,
            path: d.path,
            config: d.config,
        }
    }
}
