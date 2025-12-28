use crate::conf::types::{LoadBalancingStrategy, RouteConfig, RouteTarget, ServiceConfig};
use crate::conf::{RuntimeConfig, load_config};
use crate::device::core::registry::DeviceRegistry;
use crate::route::{RouteKind, Router};
use ahash::RandomState;
use anyhow::{Result, anyhow};
use arc_swap::ArcSwap;
use http::Uri;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

pub struct RuntimeState {
    pub router: Router,
    pub devices: DeviceRegistry,
    pub services: HashMap<String, ServiceRuntime>,
}

/// ServiceRuntime encapsulates the state of a service, including its upstream(s) and load balancing strategy.
/// It is not just a collection of data, but also a behavioral unit distinct from RuntimeState.
pub struct ServiceRuntime {
    pub strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamRuntime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct UpstreamId(pub u32);

// Fixed-seed ahash:
// - deterministic across restarts
// - fast
// - not used for security
fn make_upstream_id(host: &str, port: u16) -> UpstreamId {
    static HASHER: RandomState = RandomState::with_seeds(1, 2, 3, 4);

    let mut hasher = HASHER.build_hasher();
    (host, port).hash(&mut hasher);

    UpstreamId(hasher.finish() as u32)
}

#[derive(Debug, Clone)]
pub struct UpstreamRuntime {
    pub id: UpstreamId,
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub sni: String,
}

pub async fn reload_runtime_state(config_path: &Path, state: &ArcSwap<RuntimeState>) -> Result<()> {
    // Parse and validate config.
    let cfg = load_config(config_path)?;

    // Build a new runtime state OFFLINE.
    let new_state = build_runtime_state(&cfg)?;

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
                let rt = parse_upstream_url(&u.url)?;
                Ok(rt)
            })
            .collect::<Result<Vec<_>>>()?;

        out.insert(
            name.clone(),
            ServiceRuntime {
                strategy: svc.strategy.clone(),
                upstreams,
            },
        );
    }

    Ok(out)
}

/// Build router from config routes.
pub fn build_runtime_router(routes: &[RouteConfig]) -> anyhow::Result<Router> {
    let mut router = Router::new();

    for route in routes {
        let route_kind = match &route.target {
            RouteTarget::Service { name: service } => RouteKind::Proxy {
                upstream: service.clone(),
                allow_websocket: route.allow_websocket,
            },

            RouteTarget::Static {
                dir,
                index,
                directory_listing,
                static_config,
                cache_policy,
            } => RouteKind::Static {
                path: route.path.clone(),
                file_dir: dir.into(),
                index: index.clone().is_some(),
                directory_listing: *directory_listing,
                static_config: static_config.clone(),
                cache_policy: cache_policy.clone(),
            },
        };

        router.add_route(&route.path, route_kind)?;
    }

    Ok(router)
}

/// Parse an upstream address of the form "host:port".
fn parse_upstream_url(raw: &str) -> Result<UpstreamRuntime> {
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
    })
}
