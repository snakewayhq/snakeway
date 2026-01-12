use crate::conf::discover::discover;
use crate::conf::lower::lower_configs;
use crate::conf::parse::{parse_devices, parse_ingress};
use crate::conf::types::{
    DeviceSpec, EntrypointSpec, IngressSpec, Origin, RuntimeConfig, ServerSpec,
};
use crate::conf::validation::ValidatedConfig;
use crate::conf::validation::{ConfigError, validate_spec};

use std::fs;
use std::path::Path;

pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError> {
    let (server_spec, device_specs, ingresses) = load_spec_config(root)?;

    // Semantic validation of DSL config (aggregate all semantic errors)
    let validation_report = validate_spec(&server_spec, &ingresses, &device_specs);

    // Convert spec to runtime config.
    let (server, listeners, routes, services, devices) =
        lower_configs(server_spec, ingresses, device_specs)?;

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
        validation_report,
    })
}

pub type Spec = (ServerSpec, Vec<DeviceSpec>, Vec<IngressSpec>);

pub fn load_spec_config(root: &Path) -> Result<Spec, ConfigError> {
    //--------------------------------------------------------------------------
    // Hard fail: IO and parsing
    //--------------------------------------------------------------------------
    let root_path = root.join("snakeway.hcl");
    let entry = fs::read_to_string(&root_path).map_err(|e| ConfigError::ReadFile {
        path: root.to_path_buf(),
        source: e,
    })?;

    let mut entry: EntrypointSpec = hcl::from_str(&entry).map_err(|e| ConfigError::Parse {
        path: root_path.to_path_buf(),
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
    let mut parsed_devices: Vec<DeviceSpec> = Vec::new();
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
