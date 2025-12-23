use crate::config::{RouteConfig, SnakewayConfig};
use crate::device::core::registry::DeviceRegistry;
use crate::route::{RouteKind, Router};
use anyhow::Result;
use arc_swap::ArcSwap;
use std::path::Path;
use std::sync::Arc;

pub struct RuntimeState {
    pub router: Router,
    pub devices: DeviceRegistry,
}

pub async fn reload_runtime_state(config_path: &Path, state: &ArcSwap<RuntimeState>) -> Result<()> {
    let cfg = SnakewayConfig::from_file(config_path.to_str().expect("valid path"))?;
    let new_state = build_runtime_state(&cfg)?;

    let old = state.load();
    tracing::info!(
        old_routes = old.router.route_count(),
        old_devices = old.devices.all().len(),
        new_routes = new_state.router.route_count(),
        new_devices = new_state.devices.all().len(),
        "runtime state reloaded"
    );

    state.store(Arc::new(new_state));

    Ok(())
}

pub fn build_runtime_state(cfg: &SnakewayConfig) -> Result<RuntimeState> {
    let router = build_router(&cfg.routes)?;

    let mut registry = DeviceRegistry::new();
    registry.load_from_config(cfg)?;

    Ok(RuntimeState {
        router,
        devices: registry,
    })
}

/// Build router from config routes.
pub fn build_router(routes: &[RouteConfig]) -> anyhow::Result<Router> {
    let mut router = Router::new();

    for route in routes {
        let kind = if let Some(upstream) = &route.upstream {
            RouteKind::Proxy {
                upstream: upstream.clone(),
            }
        } else if let Some(dir) = &route.file_dir {
            RouteKind::Static {
                path: route.path.clone(),
                file_dir: dir.into(),
                index: route.index,
                directory_listing: route.directory_listing,
                static_config: route.static_config.clone(),
                cache_policy: route.cache_policy.clone(),
            }
        } else {
            unreachable!("route validation should prevent this");
        };

        router.add_route(&route.path, kind)?;
    }

    Ok(router)
}
