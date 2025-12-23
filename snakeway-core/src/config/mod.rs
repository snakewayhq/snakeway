pub mod device;
mod route;
mod server;

use anyhow::Context;
pub use device::{BuiltinDeviceKind, DeviceConfig, DeviceKind};
pub use route::{RouteConfig, StaticCachePolicy, StaticFileConfig};
use serde::Deserialize;
use server::ServerConfig;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use tracing::{debug, trace};

#[derive(Debug, Deserialize)]
pub struct SnakewayConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

impl SnakewayConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let path = Path::new(path);

        debug!(
            path = %path.display(),
            "loading snakeway config"
        );

        // Read file...
        let contents = fs::read_to_string(path).map_err(|e| {
            debug!(
                path = %path.display(),
                error = %e,
                "failed to read config file"
            );
            e
        })?;

        trace!(
            path = %path.display(),
            bytes = contents.len(),
            "config file read successfully"
        );

        // Parse...
        let cfg: Self = toml::from_str(&contents).map_err(|e| {
            debug!(
                path = %path.display(),
                error = %e,
                "failed to parse toml config"
            );
            e
        })?;

        debug!(
            routes = cfg.routes.len(),
            devices = cfg.devices.len(),
            "config parsed successfully"
        );

        // Validate...
        cfg.validate().map_err(|e| {
            debug!(
                path = %path.display(),
                error = %e,
                "config validation failed"
            );
            e
        })?;

        debug!(
            path = %path.display(),
            "snakeway config validated successfully"
        );

        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        trace!("validating snakeway config");

        for (idx, route) in self.routes.iter().enumerate() {
            trace!(
                route_index = idx,
                path = %route.path,
                "validating route"
            );

            route.validate().map_err(|e| {
                debug!(
                    route_index = idx,
                    path = %route.path,
                    error = %e,
                    "route validation failed"
                );
                e
            })?;
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
