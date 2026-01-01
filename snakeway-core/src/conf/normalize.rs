use crate::conf::types::{
    NormalizedRoute, ParsedRoute, ParsedRouteType, RouteConfig, RouteKind, StaticCachePolicy,
    StaticFileConfig,
};
use crate::conf::validation::error::ConfigError;

pub fn compile_routes(routes: Vec<NormalizedRoute>) -> Vec<RouteConfig> {
    routes
        .into_iter()
        .map(|r| RouteConfig {
            path: r.path,
            kind: r.kind,
            allow_websocket: r.allow_websocket,
            ws_idle_timeout_ms: r.ws_idle_timeout_ms,
            ws_max_connections: r.ws_max_connections,
        })
        .collect()
}

pub fn normalize_routes(routes: Vec<ParsedRoute>) -> Result<Vec<NormalizedRoute>, ConfigError> {
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

                RouteKind::Static {
                    dir,
                    index: parsed.index,
                    directory_listing: parsed.directory_listing,
                    static_config: StaticFileConfig::default(),
                    cache_policy: StaticCachePolicy::default(),
                }
            }
        };

        out.push(NormalizedRoute {
            path,
            kind,
            allow_websocket: parsed.allow_websocket,
            ws_idle_timeout_ms: parsed.ws_idle_timeout_ms,
            ws_max_connections: parsed.ws_max_connections,
        });
    }

    Ok(out)
}
