use crate::conf::types::{
    DeviceConfig, RouteConfig, ServiceConfig, ServiceRouteConfig, StaticRouteConfig,
};
use crate::conf::validation::error::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct StaticRoutesFile {
    pub route: Vec<StaticRouteConfig>,
}

pub fn parse_static_routes(path: &Path) -> Result<Vec<RouteConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: StaticRoutesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;
    let routes = parsed.route.into_iter().map(RouteConfig::Static).collect();
    Ok(routes)
}

#[derive(Deserialize)]
pub struct ServiceRoutesFile {
    pub route: Vec<ServiceRouteConfig>,
}

pub fn parse_service_routes(path: &Path) -> Result<Vec<RouteConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: ServiceRoutesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;
    let routes = parsed.route.into_iter().map(RouteConfig::Service).collect();
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
