use crate::conf::types::{RouteConfig, ServiceConfig};
use crate::conf::{RuntimeConfig, load_config};
use crate::device::core::registry::DeviceRegistry;
use crate::route::types::RouteId;
use crate::route::{RouteRuntime, Router};
use crate::runtime::{RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime};
use ahash::RandomState;
use anyhow::{Result, anyhow};
use arc_swap::ArcSwap;
use http::Uri;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub async fn reload_runtime_state(config_path: &Path, state: &ArcSwap<RuntimeState>) -> Result<()> {
    // Parse and validate config.
    let cfg = load_config(config_path)?;

    // Build a new runtime state OFFLINE.
    let new_state = build_runtime_state(&cfg.config)?;

    // Log comparison against current state.
    let old = state.load();
    tracing::info!(
        old_routes = old.router.route_count(),
        old_devices = old.devices.all().len(),
        new_routes = new_state.router.route_count(),
        new_devices = new_state.devices.all().len(),
        "runtime state reloaded"
    );

    // Atomic swap (point of no return).
    state.store(Arc::new(new_state));

    Ok(())
}

pub fn build_runtime_state(cfg: &RuntimeConfig) -> Result<RuntimeState> {
    // Router
    let router = build_runtime_router(&cfg.routes)?;

    // Devices
    let mut devices = DeviceRegistry::new();
    devices.load_from_config(cfg)?;
    tracing::debug!("Loaded device count = {}", devices.all().len());

    // Services
    let services = build_runtime_services(&cfg.services)?;

    Ok(RuntimeState {
        router,
        devices,
        services,
    })
}

fn build_runtime_services(
    services: &HashMap<String, ServiceConfig>,
) -> Result<HashMap<String, ServiceRuntime>> {
    let mut out = HashMap::new();

    for (name, svc) in services {
        let upstreams = svc
            .upstream
            .iter()
            .map(|u| {
                let rt = make_upstream_runtime(&u.url, u.weight)?;
                Ok(rt)
            })
            .collect::<Result<Vec<_>>>()?;

        out.insert(
            name.clone(),
            ServiceRuntime {
                strategy: svc.strategy.clone(),
                upstreams,
                circuit_breaker_cfg: svc.circuit_breaker.clone(),
                health_check_cfg: svc.health_check.clone(),
            },
        );
    }

    Ok(out)
}

/// Build router from config routes.
pub fn build_runtime_router(routes: &[RouteConfig]) -> anyhow::Result<Router> {
    let mut router = Router::new();

    for route in routes {
        let route_runtime = match &route {
            RouteConfig::Service(cfg) => RouteRuntime::Service {
                id: RouteId::service(&cfg.path, &cfg.service),
                upstream: cfg.service.clone(),
                allow_websocket: cfg.allow_websocket,
                ws_max_connections: cfg.ws_max_connections,
                ws_idle_timeout_ms: cfg.ws_idle_timeout_ms,
            },
            RouteConfig::Static(cfg) => RouteRuntime::Static {
                id: RouteId::static_route(&cfg.path, &canonicalize_dir(&cfg.file_dir)),
                path: cfg.path.clone(),
                file_dir: cfg.file_dir.clone(),
                index: cfg.index.is_some(),
                directory_listing: cfg.directory_listing,
                static_config: cfg.static_config.clone(),
                cache_policy: cfg.cache_policy.clone(),
            },
        };

        router.add_route(route.path(), route_runtime)?;
    }

    Ok(router)
}

/// Parse an upstream address of the form "host:port".
fn make_upstream_runtime(raw: &str, weight: u32) -> Result<UpstreamRuntime> {
    let uri: Uri = raw
        .parse()
        .map_err(|_| anyhow!("invalid upstream URL: {}", raw))?;

    let scheme = uri.scheme_str().unwrap_or("http");

    let authority = uri
        .authority()
        .ok_or_else(|| anyhow!("upstream URL missing authority: {}", raw))?;

    let host = authority.host().to_string();

    let port = authority.port_u16().unwrap_or(match scheme {
        "https" => 443,
        _ => 80,
    });

    let use_tls = scheme == "https";
    let sni = host.clone();

    Ok(UpstreamRuntime {
        id: make_upstream_id(&host, port),
        host,
        port,
        use_tls,
        sni,
        weight,
    })
}

// Fixed-seed ahash:
// - deterministic across restarts
// - fast
// - not used for security
fn make_upstream_id(host: &str, port: u16) -> UpstreamId {
    static HASHER: RandomState = RandomState::with_seeds(1, 2, 3, 4);

    UpstreamId(HASHER.hash_one((host, port)) as u32)
}

/// Converts a directory path to its full absolute path as a string.
///
/// Takes a path that might be relative (like `./files` or `../data`) and converts
/// it to a complete path (like `/home/user/app/files`). If the path doesn't exist
/// or can't be resolved, it just uses the path as-is.
fn canonicalize_dir(dir: &Path) -> String {
    let path_buf = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let result = path_buf.to_string_lossy();
    result.to_string()
}
