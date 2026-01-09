use crate::conf::types::{
    BindAdminConfig, BindConfig, DeviceConfig, ExposeRedirectConfig, ExposeServiceConfig,
    ExposeStaticConfig, IdentityDeviceConfig, IngressConfig, StructuredLoggingDeviceConfig,
    WasmDeviceConfig,
};
use crate::conf::validation::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
struct DevicesFile {
    identity_device: Option<IdentityDeviceConfig>,
    structured_logging_device: Option<StructuredLoggingDeviceConfig>,

    #[serde(default)]
    wasm_devices: Vec<WasmDeviceConfig>,
}

pub fn parse_devices(path: &Path) -> Result<Vec<DeviceConfig>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: DevicesFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    let mut device_config = Vec::new();

    if let Some(identity) = parsed.identity_device {
        device_config.push(DeviceConfig::Identity(identity));
    }

    if let Some(logging) = parsed.structured_logging_device {
        device_config.push(DeviceConfig::StructuredLogging(logging));
    }

    device_config.extend(parsed.wasm_devices.into_iter().map(DeviceConfig::Wasm));

    Ok(device_config)
}

#[derive(Debug, Deserialize)]
struct ExposeServiceFile {
    bind: Option<BindConfig>,

    bind_admin: Option<BindAdminConfig>,

    #[serde(default)]
    redirects: Vec<ExposeRedirectConfig>,

    #[serde(default)]
    services: Vec<ExposeServiceConfig>,

    #[serde(default)]
    static_files: Vec<ExposeStaticConfig>,
}

pub fn parse_ingress(path: &Path) -> Result<IngressConfig, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: ExposeServiceFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    Ok(IngressConfig {
        bind: parsed.bind,
        bind_admin: parsed.bind_admin,
        redirect_cfgs: parsed.redirects,
        service_cfgs: parsed.services,
        static_cfgs: parsed.static_files,
    })
}
