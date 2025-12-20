mod device;
mod route;
mod server;

use anyhow::Context;
pub use device::{BuiltinDeviceKind, DeviceConfig, DeviceKind};
pub use route::{RouteConfig, StaticCachePolicy, StaticFileConfig};
use serde::Deserialize;
use server::ServerConfig;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct SnakewayConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

impl SnakewayConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&contents)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for route in &self.routes {
            route.validate()?;
        }
        Ok(())
    }
}

impl FromStr for SnakewayConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let cfg: Self = toml::from_str(s).context("failed to parse Snakeway config from string")?;

        Ok(cfg)
    }
}
