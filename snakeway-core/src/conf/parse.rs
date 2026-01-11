use crate::conf::types::{
    BindAdminSpec, BindSpec, DeviceSpec, IdentityDeviceSpec, IngressSpec, Origin, RedirectSpec,
    ServiceSpec, StaticFilesSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use crate::conf::validation::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
struct DevicesFile {
    identity_device: Option<IdentityDeviceSpec>,
    structured_logging_device: Option<StructuredLoggingDeviceSpec>,

    #[serde(default)]
    wasm_devices: Vec<WasmDeviceSpec>,
}

pub fn parse_devices(path: &Path) -> Result<Vec<DeviceSpec>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: DevicesFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    let mut device_config = Vec::new();

    if let Some(identity) = parsed.identity_device {
        device_config.push(DeviceSpec::Identity(identity));
    }

    if let Some(logging) = parsed.structured_logging_device {
        device_config.push(DeviceSpec::StructuredLogging(logging));
    }

    device_config.extend(parsed.wasm_devices.into_iter().map(DeviceSpec::Wasm));

    Ok(device_config)
}

#[derive(Debug, Deserialize)]
struct IngressFile {
    bind: Option<BindSpec>,

    bind_admin: Option<BindAdminSpec>,

    #[serde(default)]
    redirects: Vec<RedirectSpec>,

    #[serde(default)]
    services: Vec<ServiceSpec>,

    #[serde(default)]
    static_files: Vec<StaticFilesSpec>,
}

pub fn parse_ingress(path: &Path) -> Result<IngressSpec, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let mut parsed: IngressFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

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
        for (j, backend) in service.upstreams.iter_mut().enumerate() {
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

    Ok(IngressSpec {
        bind: parsed.bind,
        bind_admin: parsed.bind_admin,
        redirect_cfgs: parsed.redirects,
        service_cfgs: parsed.services,
        static_cfgs: parsed.static_files,
    })
}
