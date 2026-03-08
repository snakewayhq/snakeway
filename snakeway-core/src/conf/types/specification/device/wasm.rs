use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct WasmDeviceSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,

    pub(crate) enable: bool,

    /// The location of the WASM module.
    pub(crate) path: PathBuf,

    /// Device-specific configuration blob
    pub(crate) config: Option<hcl::Value>,
}
