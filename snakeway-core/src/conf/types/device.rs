use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    pub name: String,

    #[serde(rename = "type")]
    pub kind: DeviceKind,

    /// Device-specific configuration blob
    #[serde(default)]
    pub config: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Builtin,
    Wasm,
}
