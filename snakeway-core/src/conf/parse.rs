use crate::conf::types::{
    DeviceConfig, RouteConfig, ServiceConfig, ServiceRouteConfig, StaticRouteConfig,
};
use crate::conf::validation::error::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RoutesFile {
    #[serde(default)]
    pub service_route: Vec<ServiceRouteConfig>,

    #[serde(default)]
    pub static_route: Vec<StaticRouteConfig>,
}

pub fn parse_routes(path: &Path) -> Result<Vec<RouteConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: RoutesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;
    let mut routes = Vec::new();
    routes.extend(parsed.service_route.into_iter().map(RouteConfig::Service));
    routes.extend(parsed.static_route.into_iter().map(RouteConfig::Static));
    Ok(routes)
}

#[derive(Debug, Deserialize)]
struct ServicesFile {
    #[serde(default)]
    service: Vec<ServiceConfig>,
}

pub fn parse_services(path: &Path) -> Result<Vec<ServiceConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;

    let parsed: ServicesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    Ok(parsed.service)
}

#[derive(Debug, Deserialize)]
struct DevicesFile {
    #[serde(default)]
    device: Vec<DeviceConfig>,
}

pub fn parse_devices(paths: Vec<std::path::PathBuf>) -> Result<Vec<DeviceConfig>, ConfigError> {
    let mut devices = Vec::new();

    for path in paths {
        let s = fs::read_to_string(&path).map_err(|e| ConfigError::read_file(&path, e))?;

        let parsed: DevicesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(&path, e))?;

        devices.extend(parsed.device);
    }

    Ok(devices)
}
