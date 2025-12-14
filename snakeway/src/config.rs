use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// e.g. "0.0.0.0:8080"
    pub listen: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    /// e.g. "127.0.0.1:3000"
    pub upstream: String,
}

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
    pub (crate) options: toml::Value,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SnakewayConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    pub devices: Vec<DeviceConfig>,
}

impl SnakewayConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}
