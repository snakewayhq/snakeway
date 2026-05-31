use crate::types::{HclInt, HclOrigin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

fn default_timeout_ms() -> HclInt {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WasmDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

    pub name: String,

    pub enable: bool,

    /// The location of the WASM module.
    pub path: PathBuf,

    /// Behavior on guest trap, timeout, or load error: "open" or "closed".
    pub fail_policy: WasmDeviceFailPolicy,

    /// Per-hook epoch deadline in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: HclInt,

    /// Max body buffer bytes. 0 = streaming.
    #[serde(default)]
    pub body_buffer_max: HclInt,

    /// Arbitrary key-value config passed to the guest via host.config-get.
    #[serde(default)]
    pub config: HashMap<String, String>,
}

impl Default for WasmDeviceSpec {
    fn default() -> Self {
        Self {
            origin: HclOrigin::default(),
            name: String::new(),
            enable: false,
            path: PathBuf::new(),
            fail_policy: WasmDeviceFailPolicy::Open,
            timeout_ms: default_timeout_ms(),
            body_buffer_max: 0,
            config: HashMap::new(),
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WasmDeviceFailPolicy {
    #[default]
    Open,
    Closed,
}
