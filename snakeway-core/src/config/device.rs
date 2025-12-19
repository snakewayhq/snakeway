use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Wasm,
    Builtin,
}

#[derive(Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinDeviceKind {
    StructuredLogging,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    pub name: String,

    pub kind: DeviceKind,

    /// Required for `kind = "wasm"`
    pub path: Option<String>,

    /// Required for `kind = "builtin"`
    pub builtin: Option<BuiltinDeviceKind>,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(flatten)]
    pub(crate) options: toml::Value,
}

fn default_enabled() -> bool {
    true
}
