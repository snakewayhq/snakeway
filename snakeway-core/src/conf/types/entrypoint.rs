use crate::conf::types::ServerConfig;
use crate::conf::types::listener::ListenerConfig;
use serde::Deserialize;

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize)]
pub struct EntrypointConfig {
    pub server: ServerConfig,
    #[serde(rename = "listener")]
    pub listeners: Vec<ListenerConfig>,
    pub include: IncludeConfig,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize)]
pub struct IncludeConfig {
    pub static_routes: String,
    pub service_routes: String,
    pub services: String,
    pub devices: String,
}
