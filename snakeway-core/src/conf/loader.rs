use crate::conf::discover::discover;
use crate::conf::lower::lower_configs;
use crate::conf::parse::{parse_devices, parse_ingress};
use crate::conf::types::{
    DeviceConfig, EntrypointConfig, ExposeServerConfig, IngressConfig, Origin, RuntimeConfig,
};
use crate::conf::validation::ValidatedConfig;
use crate::conf::validation::{ConfigError, validate_dsl_config};

use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError> {
    //--------------------------------------------------------------------------
    // Semantic validation of DSL config (aggregate all semantic errors)
    //--------------------------------------------------------------------------
    let (server_dsl, devices, ingresses) = load_dsl_config(root)?;
    let dsl_validation = validate_dsl_config(&server_dsl, &ingresses, &devices);

    //--------------------------------------------------------------------------
    // Semantic validation of IR config (aggregate all semantic errors)
    //--------------------------------------------------------------------------
    let (server, listeners, routes, services) = lower_configs(server_dsl, ingresses)?;

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
        dsl_validation,
    })
}

pub type DslRepresentation = (ExposeServerConfig, Vec<DeviceConfig>, Vec<IngressConfig>);

pub fn load_dsl_config(root: &Path) -> Result<DslRepresentation, ConfigError> {
    //--------------------------------------------------------------------------
    // Hard fail: IO and parsing
    //--------------------------------------------------------------------------
    let root_path = root.join("snakeway.hcl");
    let entry = fs::read_to_string(&root_path).map_err(|e| ConfigError::ReadFile {
        path: root.to_path_buf(),
        source: e,
    })?;

    let mut entry: EntrypointConfig = hcl::from_str(&entry).map_err(|e| ConfigError::Parse {
        path: root.to_path_buf(),
        source: e,
    })?;

    entry.server.origin = Origin::new(&root_path, "server", None);

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
