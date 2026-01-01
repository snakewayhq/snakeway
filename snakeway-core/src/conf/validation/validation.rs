use crate::conf::types::{EntrypointConfig, RouteConfig, ServiceConfig};
use crate::conf::validation::validation_ctx::{ValidationCtx, ValidationErrors};
use crate::conf::validation::validators;
use std::collections::HashMap;

/// Validate everything that exists in a fully parsed config.
pub fn validate_runtime_config(
    entry: &EntrypointConfig,
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
) -> Result<(), ValidationErrors> {
    let mut ctx = ValidationCtx::default();

    if let Err(e) = validators::validate_version(entry.server.version) {
        ctx.push(e);
    }

    validators::validate_listeners(&entry.listeners, &mut ctx);
    validators::validate_routes(routes, &mut ctx);
    validators::validate_services(services, &mut ctx);

    ctx.into_result()
}
