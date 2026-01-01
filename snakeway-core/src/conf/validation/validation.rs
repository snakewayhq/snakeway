use crate::conf::types::{
    EntrypointConfig, ParsedRoute, ParsedRouteType, RouteConfig, RouteKind, ServiceConfig,
    StaticCachePolicy, StaticFileConfig,
};
use crate::conf::validation::error::ConfigError;
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
    validators::validate_routes(routes, services, &mut ctx);
    validators::validate_services(services, &mut ctx);

    ctx.into_result()
}

/// Compile parsed routes into finalized RouteConfig values.
///
/// Still fail-fast: malformed route declarations cannot be meaningfully validated.
pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for parsed in routes {
        let path = match parsed.path.trim_end_matches('/') {
            "" => "/".to_string(),
            p => p.to_string(),
        };

        let kind = match parsed.r#type {
            ParsedRouteType::Service => {
                let service =
                    parsed
                        .service
                        .ok_or_else(|| ConfigError::MissingServiceForServiceRoute {
                            path: path.clone(),
                        })?;

                if parsed.file_dir.is_some() {
                    return Err(ConfigError::DirNotAllowedOnServiceRoute { path });
                }

                RouteKind::Service { name: service }
            }

            ParsedRouteType::Static => {
                let dir = parsed
                    .file_dir
                    .ok_or_else(|| ConfigError::MissingDirForStaticRoute { path: path.clone() })?;

                if parsed.service.is_some() {
                    return Err(ConfigError::ServiceNotAllowedOnStaticRoute { path });
                }

                if parsed.allow_websocket {
                    return Err(ConfigError::WebSocketNotAllowedOnStaticRoute { path });
                }

                RouteKind::Static {
                    dir,
                    index: parsed.index,
                    directory_listing: parsed.directory_listing,
                    static_config: StaticFileConfig::default(),
                    cache_policy: StaticCachePolicy::default(),
                }
            }
        };

        out.push(RouteConfig {
            path,
            kind,
            allow_websocket: parsed.allow_websocket,
            ws_idle_timeout_ms: parsed.ws_idle_timeout_ms,
            ws_max_connections: parsed.ws_max_connections,
        });
    }

    Ok(out)
}
