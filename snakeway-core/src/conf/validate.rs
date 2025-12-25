use crate::conf::error::ConfigError;
use crate::conf::types::{
    ParsedRoute, RouteConfig, RouteTarget, ServiceConfig, StaticCachePolicy, StaticFileConfig,
};
use std::collections::HashMap;

pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
) -> Result<(), ConfigError> {
    for route in routes {
        let RouteTarget::Service { name } = &route.target else {
            continue;
        };

        if !services.contains_key(name) {
            return Err(ConfigError::UnknownService {
                route: route.path.clone(),
                service: name.clone(),
            });
        }
    }

    Ok(())
}

pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for r in routes {
        let target = match (r.service, r.file_dir) {
            (Some(service), None) => RouteTarget::Service { name: service },

            (None, Some(dir)) => {
                let static_config = StaticFileConfig::default();
                let cache_policy = StaticCachePolicy::default();
                RouteTarget::Static {
                    dir,
                    index: r.index,
                    directory_listing: r.directory_listing,
                    static_config,
                    cache_policy,
                }
            }

            _ => {
                return Err(ConfigError::InvalidRoute { path: r.path });
            }
        };

        out.push(RouteConfig {
            path: r.path,
            target,
        });
    }

    Ok(out)
}
