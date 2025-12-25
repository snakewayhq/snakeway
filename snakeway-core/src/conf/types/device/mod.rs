pub mod identity;
pub mod structured_logging;
pub use identity::*;
use serde::Deserialize;
pub use structured_logging::*;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    pub name: String,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(rename = "type")]
    pub kind: DeviceKind,

    /// Required for `kind = "builtin"`
    pub builtin: Option<BuiltinDeviceKind>,

    /// Required for `kind = "wasm"`
    pub path: Option<String>,

    /// Device-specific configuration blob
    pub config: toml::Value,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Builtin,
    Wasm,
}

#[derive(Debug, Deserialize, Eq, Hash, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinDeviceKind {
    StructuredLogging,
    Identity,
}
