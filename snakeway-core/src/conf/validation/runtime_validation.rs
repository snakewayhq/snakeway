use crate::conf::types::{DeviceConfig, ListenerConfig, RouteConfig, ServerConfig, ServiceConfig};
use crate::conf::validation::validation_ctx::{ValidationCtx, ValidationErrors};
use crate::conf::validation::{ValidationOutput, validator};
use std::collections::HashMap;

/// Validate everything that exists in a fully parsed config.
pub fn validate_runtime_config(
    server: &ServerConfig,
    listeners: &[ListenerConfig],
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
    devices: &Vec<DeviceConfig>,
) -> Result<ValidationOutput, ValidationErrors> {
    let mut ctx = ValidationCtx::default();

    if let Err(e) = validator::validate_version(server.version) {
        ctx.error(e);
    } else {
        validator::validate_server(server, &mut ctx);
        validator::validate_listeners(listeners, &mut ctx);
        validator::validate_routes(routes, services, &mut ctx);
        validator::validate_services(services, &mut ctx);
        validator::validate_devices(devices, &mut ctx);
    }

    ctx.into_result()
}
