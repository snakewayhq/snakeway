use crate::conf::discover::discover;
use crate::conf::lower::lower_expose_configs;
use crate::conf::parse::{parse_devices, parse_ingress};
use crate::conf::runtime::{RuntimeConfig, ValidatedConfig};
use crate::conf::types::{DeviceConfig, EntrypointConfig, ExposeConfig};
use crate::conf::validation::ConfigError;
use crate::conf::validation::validate_runtime_config;
use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError> {
    //--------------------------------------------------------------------------
    // Hard fail: IO and parsing
    //--------------------------------------------------------------------------
    let entry =
        fs::read_to_string(root.join("snakeway.toml")).map_err(|e| ConfigError::ReadFile {
            path: root.to_path_buf(),
            source: e,
        })?;

    let entry: EntrypointConfig = toml::from_str(&entry).map_err(|e| ConfigError::Parse {
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
    let exposes: Vec<ExposeConfig> = ingress_files
        .iter()
        .map(|p| parse_ingress(p.as_path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    let (listeners, routes, services) = lower_expose_configs(exposes)?;

    //--------------------------------------------------------------------------
    // Semantic validation (aggregate all semantic errors)
    //--------------------------------------------------------------------------
    let validation = validate_runtime_config(
        &entry.server,
        &listeners,
        &routes,
        &services,
        &parsed_devices,
    )
    .map_err(|errs| ConfigError::Validation {
        validation_errors: errs,
    })?;

    //--------------------------------------------------------------------------
    // Build runtime config
    //--------------------------------------------------------------------------
    Ok(ValidatedConfig {
        config: RuntimeConfig {
            server: entry.server,
            devices: parsed_devices,
            routes,
            services,
            listeners,
        },
        validation,
    })
}
