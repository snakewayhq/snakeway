use crate::conf::types::WasmDeviceSpec;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct WasmDeviceConfig {
    pub(crate) enable: bool,

    /// The location of the WASM module.
    pub(crate) path: PathBuf,

    /// Device-specific configuration blob
    pub(crate) config: Option<hcl::Value>,
}

impl From<WasmDeviceSpec> for WasmDeviceConfig {
    fn from(spec: WasmDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            path: spec.path,
            config: spec.config,
        }
    }
}
