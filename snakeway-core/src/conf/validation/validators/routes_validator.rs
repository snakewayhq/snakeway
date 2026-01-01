use crate::conf::types::{RouteConfig, ServiceConfig};
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;
use std::collections::HashMap;

/// Validate routes and referenced services.
pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
    ctx: &mut ValidationCtx,
) {
    for route in routes {
        match route {
            RouteConfig::Service(cfg) => {
                if !services.contains_key(&cfg.service) {
                    ctx.push(ConfigError::UnknownService {
                        path: cfg.path.clone(),
                        service: cfg.service.clone(),
                    });
                }
            }
            RouteConfig::Static(cfg) => {
                if !cfg.file_dir.exists() {
                    ctx.push(ConfigError::InvalidStaticDir {
                        path: cfg.file_dir.clone(),
                        reason: "does not exist".to_string(),
                    });
                }
            }
        };
    }
}
