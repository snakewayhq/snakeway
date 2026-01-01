use crate::conf::types::{RouteConfig, RouteKind};
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;

/// Validate routes and referenced services.
pub fn validate_routes(routes: &[RouteConfig], ctx: &mut ValidationCtx) {
    for route in routes {
        match &route.kind {
            RouteKind::Static { .. } if route.allow_websocket => {
                ctx.push(ConfigError::WebSocketNotAllowedOnStaticRoute {
                    path: route.path.clone(),
                });
            }
            _ => {}
        }
    }
}
