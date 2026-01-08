use crate::conf::types::{
    AdminBindConfig, BindConfig, DeviceConfig, ExposeRedirectConfig, ExposeServiceConfig,
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
    wasm_device: Vec<WasmDeviceConfig>,
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

    device_config.extend(parsed.wasm_device.into_iter().map(DeviceConfig::Wasm));

    Ok(device_config)
}

#[derive(Debug, Deserialize)]
struct ExposeServiceFile {
    bind: Option<BindConfig>,

    admin_bind: Option<AdminBindConfig>,

    #[serde(default)]
    expose_redirect: Vec<ExposeRedirectConfig>,

    #[serde(default)]
    expose_service: Vec<ExposeServiceConfig>,

    #[serde(default)]
    expose_static: Vec<ExposeStaticConfig>,
}

pub fn parse_ingress(path: &Path) -> Result<IngressConfig, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: ExposeServiceFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    Ok(IngressConfig {
        bind: parsed.bind,
        admin_bind: parsed.admin_bind,
        redirect_cfgs: parsed.expose_redirect,
        service_cfgs: parsed.expose_service,
        static_cfgs: parsed.expose_static,
    })
}
