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
pub struct SnakewayConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
}

impl SnakewayConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}
