use crate::conf::types::{DeviceSpec, DevicesFile, IngressFile, IngressSpec, Origin};
use crate::conf::validation::ConfigError;
use std::fs;
use std::path::Path;

pub fn parse_devices(path: &Path) -> Result<Vec<DeviceSpec>, ConfigError> {
    let s = fs::read_to_string(path).map_err(|e| ConfigError::read_file(path, e))?;
    let parsed: DevicesFile = hcl::from_str(&s).map_err(|e| ConfigError::parse(path, e))?;

    let mut device_config = Vec::new();

    if let Some(mut identity) = parsed.identity_device {
        identity.origin = Origin::new(&path.to_path_buf(), "identity_device", None);
        device_config.push(DeviceSpec::Identity(identity));
    }

    if let Some(mut network_policy) = parsed.network_policy_device {
        network_policy.origin = Origin::new(&path.to_path_buf(), "network_policy_device", None);
        device_config.push(DeviceSpec::NetworkPolicy(network_policy));
    }

    if let Some(mut request_rate_limiting) = parsed.request_rate_limiting_device {
        request_rate_limiting.origin =
            Origin::new(&path.to_path_buf(), "request_rate_limiting_device", None);
        device_config.push(DeviceSpec::RequestRateLimiting(request_rate_limiting));
    }

    if let Some(mut otel) = parsed.otel_device {
        otel.origin = Origin::new(&path.to_path_buf(), "otel_device", None);
        device_config.push(DeviceSpec::Otel(otel));
    }

    if let Some(mut logging) = parsed.structured_logging_device {
        logging.origin = Origin::new(&path.to_path_buf(), "structured_logging_device", None);
        device_config.push(DeviceSpec::StructuredLogging(logging));
    }

    if let Some(mut request_filter) = parsed.request_filter_device {
        request_filter.origin = Origin::new(&path.to_path_buf(), "request_filter_device", None);
        device_config.push(DeviceSpec::RequestFilter(request_filter));
    }

    for (idx, mut device) in parsed.wasm_devices.into_iter().enumerate() {
        device.origin = Origin::new(&path.to_path_buf(), "wasm_device", idx.into());
        device_config.push(DeviceSpec::Wasm(device));
    }

    Ok(device_config)
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
        origin: Origin::new(&path.to_path_buf(), "ingress", None),
        bind: parsed.bind,
        bind_admin: parsed.bind_admin,
        services: parsed.services,
        static_files: parsed.static_files,
    })
}
