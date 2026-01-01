use crate::conf::discover::{discover, resolve_glob};
use crate::conf::merge::merge_services;
use crate::conf::parse::{parse_devices, parse_routes, parse_services};
use crate::conf::runtime::RuntimeConfig;
use crate::conf::types::EntrypointConfig;
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validators::{compile_routes, validate_runtime_config};
use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<RuntimeConfig, ConfigError> {
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

    let route_files = discover(&resolve_glob(root, &entry.include.routes))?;
    let service_files = discover(&resolve_glob(root, &entry.include.services))?;
    let device_files = discover(&resolve_glob(root, &entry.include.devices))?;

    //--------------------------------------------------------------------------
    // Parse routes (hard fail)
    //--------------------------------------------------------------------------
    let mut parsed_routes = Vec::new();
    for path in &route_files {
        parsed_routes.extend(parse_routes(path.as_path())?);
    }
    let routes = compile_routes(parsed_routes)?;

    //--------------------------------------------------------------------------
    // Parse services (hard fail)
    //--------------------------------------------------------------------------
    let mut parsed_services = Vec::new();
    for path in &service_files {
        parsed_services.extend(parse_services(path.as_path())?);
    }
    let services = merge_services(parsed_services)?;

    //--------------------------------------------------------------------------
    // Semantic validation (aggregate all semantic errors)
    //--------------------------------------------------------------------------
    validate_runtime_config(&entry, &routes, &services).map_err(|errs| {
        ConfigError::Validation {
            validation_errors: errs,
        }
    })?;

    //--------------------------------------------------------------------------
    // Build runtime config
    //--------------------------------------------------------------------------
    Ok(RuntimeConfig {
        server: entry.server,
        routes,
        services,
        devices: parse_devices(device_files)?,
        listeners: entry.listeners,
    })
}
