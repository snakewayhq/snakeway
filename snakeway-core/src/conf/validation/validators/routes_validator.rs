use crate::conf::types::{RouteConfig, RouteKind};
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;

/// Validate routes and referenced services.
pub fn validate_routes(routes: &[RouteConfig], ctx: &mut ValidationCtx) {
    for route in routes {
        if let RouteKind::Static { .. } = &route.kind {
            if route.allow_websocket {
                ctx.push(ConfigError::WebSocketNotAllowedOnStaticRoute {
                    path: route.path.clone(),
                });
            }
            if route.ws_idle_timeout_ms.is_some() {
                ctx.push(ConfigError::InvalidRoute {
                    path: route.path.clone(),
                });
            }
        }
    }
}
