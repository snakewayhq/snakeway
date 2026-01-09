use crate::conf::types::ServerConfig;
use serde::Deserialize;

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize)]
pub struct EntrypointConfig {
    pub server: ServerConfig,
    pub include: IncludeConfig,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize)]
pub struct IncludeConfig {
    pub devices: String,
    pub ingress: String,
}
