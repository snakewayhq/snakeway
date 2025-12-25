use crate::conf::types::ServerConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EntrypointConfig {
    pub server: ServerConfig,

    pub include: IncludeConfig,
}

#[derive(Debug, Deserialize)]
pub struct IncludeConfig {
    pub routes: String,
    pub services: String,
    pub devices: String,
}
