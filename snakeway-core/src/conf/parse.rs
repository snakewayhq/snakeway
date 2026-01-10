use crate::conf::types::{
    BindAdminConfig, BindConfig, DeviceConfig, ExposeRedirectConfig, ExposeServiceConfig,
    ExposeStaticConfig, IdentityDeviceConfig, IngressConfig, Origin, StructuredLoggingDeviceConfig,
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
    let mut parsed: ExposeServiceFile =
        hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    //-------------------------------------------------------------------------
    // Inject origin metadata
    //-------------------------------------------------------------------------

    if let Some(bind) = &mut parsed.bind {
        bind.origin = Origin::new(&path.to_path_buf(), "bind", None);
    }

    if let Some(bind_admin) = &mut parsed.bind_admin {
        bind_admin.origin = Origin::new(&path.to_path_buf(), "bind_admin", None);
    }

    for (i, redirect) in parsed.redirects.iter_mut().enumerate() {
        redirect.origin = Origin::new(&path.to_path_buf(), "redirect", Some(i));
    }

    for (i, service) in parsed.services.iter_mut().enumerate() {
        service.origin = Origin::new(&path.to_path_buf(), "service", Some(i));
        for (j, route) in service.routes.iter_mut().enumerate() {
            route.origin = Origin::new(&path.to_path_buf(), "route", Some(j));
        }
        for (j, backend) in service.backends.iter_mut().enumerate() {
            backend.origin = Origin::new(&path.to_path_buf(), "backend", Some(j));
        }
    }

    for (i, static_files) in parsed.static_files.iter_mut().enumerate() {
        static_files.origin = Origin::new(&path.to_path_buf(), "static_files", Some(i));
        for (j, route) in static_files.routes.iter_mut().enumerate() {
            route.origin = Origin::new(&path.to_path_buf(), "route", Some(j));
        }
    }

    //-------------------------------------------------------------------------
    // Lower to ingress config
    //-------------------------------------------------------------------------

    Ok(IngressConfig {
        bind: parsed.bind,
        bind_admin: parsed.bind_admin,
        redirect_cfgs: parsed.redirects,
        service_cfgs: parsed.services,
        static_cfgs: parsed.static_files,
    })
}
