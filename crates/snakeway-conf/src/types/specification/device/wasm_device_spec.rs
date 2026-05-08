use crate::types::HclOrigin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct WasmDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

    pub enable: bool,

    /// The location of the WASM module.
    pub path: PathBuf,

    /// Device-specific configuration blob
    pub config: Option<hcl::Value>,
}
