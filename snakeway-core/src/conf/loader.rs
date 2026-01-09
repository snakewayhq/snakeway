use crate::conf::discover::discover;
use crate::conf::lower::lower_configs;
use crate::conf::parse::{parse_devices, parse_ingress};
use crate::conf::types::{
    DeviceConfig, EntrypointConfig, IngressConfig, ListenerConfig, RouteConfig, RuntimeConfig,
    ServerConfig, ServiceConfig,
};
use crate::conf::validation::ValidatedConfig;
use crate::conf::validation::{ConfigError, ValidationCtx, ValidationErrors, ValidationOutput};

use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError> {
    let (server, devices, ingresses) = load_dsl_config(root)?;

    let validation = validate_dsl_config(&server, &ingresses, &devices).map_err(|errs| {
        ConfigError::Validation {
            validation_errors: errs,
        }
    })?;

    let (listeners, routes, services) = lower_configs(ingresses)?;

    //--------------------------------------------------------------------------
    // Semantic validation (aggregate all semantic errors)
    //--------------------------------------------------------------------------
    // let validation = validate_runtime_config(&server, &listeners, &routes, &services, &devices)
    //     .map_err(|errs| ConfigError::Validation {
    //         validation_errors: errs,
    //     })?;

    //--------------------------------------------------------------------------
    // Build runtime config
    //--------------------------------------------------------------------------
    Ok(ValidatedConfig {
        config: RuntimeConfig {
            server,
            devices,
            routes,
            services,
            listeners,
        },
        validation,
    })
}

pub fn validate_dsl_config(
    server: &ServerConfig,
    ingresses: &[IngressConfig],
    devices: &Vec<DeviceConfig>,
) -> Result<ValidationOutput, ValidationErrors> {
    let mut ctx = ValidationCtx::default();

    if let Err(e) = crate::conf::validation::validator::validate_version(server.version) {
        ctx.error(e);
    } else {
        crate::conf::validation::validator::validate_server(server, &mut ctx);
        crate::conf::validation::validator::validate_ingresses(ingresses, &mut ctx);
        crate::conf::validation::validator::validate_devices(devices, &mut ctx);
    }

    ctx.into_result()
}

pub type ConfigIntermediateRepresentation = (ServerConfig, Vec<DeviceConfig>, Vec<IngressConfig>);

pub fn load_dsl_config(root: &Path) -> Result<ConfigIntermediateRepresentation, ConfigError> {
    //--------------------------------------------------------------------------
    // Hard fail: IO and parsing
    //--------------------------------------------------------------------------
    let entry =
        fs::read_to_string(root.join("snakeway.hcl")).map_err(|e| ConfigError::ReadFile {
            path: root.to_path_buf(),
            source: e,
        })?;

    let entry: EntrypointConfig = hcl::from_str(&entry).map_err(|e| ConfigError::Parse {
        path: root.to_path_buf(),
        source: e,
    })?;

    //--------------------------------------------------------------------------
    // Discover included files (hard fail)
    //--------------------------------------------------------------------------
    let device_files = discover(root, &entry.include.devices)?;
    let ingress_files = discover(root, &entry.include.ingress)?;

    //--------------------------------------------------------------------------
    // Parse devices (hard fail)
    //--------------------------------------------------------------------------
    let mut parsed_devices: Vec<DeviceConfig> = Vec::new();
    for path in &device_files {
        parsed_devices.extend(parse_devices(path.as_path())?);
    }

    //--------------------------------------------------------------------------
    // Parse ingress (hard fail)
    //--------------------------------------------------------------------------
    let ingresses = ingress_files
        .iter()
        .map(|p| parse_ingress(p.as_path()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((entry.server, parsed_devices, ingresses))
}
