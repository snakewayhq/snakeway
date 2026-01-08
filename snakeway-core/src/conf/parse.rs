use crate::conf::types::{
    DeviceConfig, ExposeConfig, ExposeRedirectConfig, ExposeServiceConfig, ExposeStaticConfig,
    IdentityDeviceConfig, RouteConfig, ServiceConfig, ServiceRouteConfig, StaticRouteConfig,
    StructuredLoggingDeviceConfig, WasmDeviceConfig,
};
use crate::conf::validation::ConfigError;
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

#[derive(Debug, Deserialize, Default)]
struct DevicesFile {
    #[serde(default)]
    identity_device: IdentityDeviceConfig,

    #[serde(default)]
    structured_logging_device: StructuredLoggingDeviceConfig,

    #[serde(default)]
    wasm_device: Vec<WasmDeviceConfig>,
}

pub fn parse_devices(path: &Path) -> Result<Vec<DeviceConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: DevicesFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;
    let mut device_config = Vec::new();

    device_config.push(DeviceConfig::Identity(parsed.identity_device.clone()));
    device_config.push(DeviceConfig::StructuredLogging(
        parsed.structured_logging_device.clone(),
    ));
    device_config.extend(parsed.wasm_device.into_iter().map(DeviceConfig::Wasm));

    Ok(device_config)
}

#[derive(Debug, Deserialize)]
struct ExposeServiceFile {
    #[serde(default)]
    expose_redirect: Vec<ExposeRedirectConfig>,

    #[serde(default)]
    expose_service: Option<ExposeServiceConfig>,

    #[serde(default)]
    expose_static: Option<ExposeStaticConfig>,
}

pub fn parse_ingress(path: &Path) -> Result<Vec<ExposeConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;

    let parsed: ExposeServiceFile = toml::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    let mut exposes = Vec::new();

    // Redirects (plural, order preserved)
    for redirect in parsed.expose_redirect {
        exposes.push(ExposeConfig::Redirect(redirect));
    }

    // Service (at most one)
    if let Some(service) = parsed.expose_service {
        exposes.push(ExposeConfig::Service(service));
    }

    // Static (at most one)
    if let Some(static_cfg) = parsed.expose_static {
        exposes.push(ExposeConfig::StaticRoute(static_cfg));
    }

    Ok(exposes)
}
