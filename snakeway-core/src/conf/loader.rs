use crate::conf::discover::discover;
use crate::conf::error::ConfigError;
use crate::conf::merge::merge_services;
use crate::conf::parse::{parse_devices, parse_routes, parse_services};
use crate::conf::runtime::RuntimeConfig;
use crate::conf::types::EntrypointConfig;
use crate::conf::validate::{
    compile_routes, validate_listeners, validate_routes, validate_version,
};
use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<RuntimeConfig, ConfigError> {
    let entry =
        fs::read_to_string(root.join("snakeway.toml")).map_err(|e| ConfigError::ReadFile {
            path: root.to_path_buf(),
            source: e,
        })?;
    let entry: EntrypointConfig = toml::from_str(&entry).map_err(|e| ConfigError::Parse {
        path: root.to_path_buf(),
        source: e,
    })?;

    validate_version(entry.server.version)?;
    validate_listeners(&entry.listeners)?;

    fn resolve_glob(root: &Path, pattern: &str) -> String {
        root.join(pattern).to_string_lossy().into_owned()
    }

    let route_files = discover(&resolve_glob(root, &entry.include.routes))?;
    let service_files = discover(&resolve_glob(root, &entry.include.services))?;
    let device_files = discover(&resolve_glob(root, &entry.include.devices))?;

    // Parse routes
    let mut parsed_routes = Vec::new();
    for path in &route_files {
        let routes = parse_routes(path.as_path())?;
        parsed_routes.extend(routes);
    }
    let routes = compile_routes(parsed_routes)?;

    // Parse services
    let mut parsed_services = Vec::new();
    for path in &service_files {
        let services = parse_services(path.as_path())?;
        parsed_services.extend(services);
    }
    let services = merge_services(parsed_services)?;

    // Validate routes and services together.
    validate_routes(&routes, &services)?;

    Ok(RuntimeConfig {
        server: entry.server,
        routes,
        services,
        devices: parse_devices(device_files)?,
        listeners: entry.listeners,
    })
}
