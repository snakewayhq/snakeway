use crate::conf::types::{RouteConfig, RouteKind, ServiceConfig};
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Validate routes and referenced services.
pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
    ctx: &mut ValidationCtx,
) {
    let mut seen_paths = HashSet::new();

    for route in routes {
        if !route.path.starts_with('/') {
            ctx.push(ConfigError::InvalidRoutePath {
                path: route.path.clone(),
                reason: "route path must start with '/'".into(),
            });
        }

        if !seen_paths.insert(route.path.clone()) {
            ctx.push(ConfigError::DuplicateRoute {
                path: route.path.clone(),
            });
        }

        match &route.kind {
            RouteKind::Service { name } => {
                if !services.contains_key(name) {
                    ctx.push(ConfigError::UnknownService {
                        route: route.path.clone(),
                        service: name.clone(),
                    });
                }
            }

            RouteKind::Static { dir, .. } => {
                if route.allow_websocket {
                    ctx.push(ConfigError::WebSocketNotAllowedOnStaticRoute {
                        path: route.path.clone(),
                    });
                }

                let dir_path = Path::new(dir);
                if dir_path.is_relative() || !dir_path.is_dir() || dir_path == Path::new("/") {
                    ctx.push(ConfigError::InvalidStaticDir {
                        path: PathBuf::from(dir),
                        reason: "static routes must point to an absolute, non-root directory"
                            .into(),
                    });
                }
            }
        }
    }
}
