use crate::conf::types::{RouteConfig, RouteTarget, ServiceConfig, Strategy};
use crate::conf::{RuntimeConfig, load_config};
use crate::device::core::registry::DeviceRegistry;
use crate::route::{RouteKind, Router};
use anyhow::{Result, anyhow};
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RuntimeState {
    pub router: Router,
    pub devices: DeviceRegistry,
    pub services: HashMap<String, ServiceRuntime>,
}

/// ServiceRuntime encapsulates the state of a service, including its upstream(s) and load balancing strategy.
/// It is not just a collection of data, but also a behavioral unit distinct from RuntimeState.
pub struct ServiceRuntime {
    pub strategy: Strategy,
    pub upstreams: Vec<UpstreamRuntime>,
    pub round_robin_cursor: AtomicUsize,
}

impl ServiceRuntime {
    pub fn select_upstream(&self) -> Option<&UpstreamRuntime> {
        if self.upstreams.is_empty() {
            return None;
        }

        match self.strategy {
            Strategy::RoundRobin => self.round_robin(),
            Strategy::Failover => self.failover(),
            Strategy::LeastConnections => self.least_connections(),
        }
    }
}

impl ServiceRuntime {
    fn round_robin(&self) -> Option<&UpstreamRuntime> {
        let len = self.upstreams.len();

        let idx = self.round_robin_cursor.fetch_add(1, Ordering::Relaxed) % len;

        self.upstreams.get(idx)
    }

    fn failover(&self) -> Option<&UpstreamRuntime> {
        // todo: handle failover strategy, there is no is_healthy yet. Will be something like: self.upstreams.iter().find(|u| u.is_healthy())
        // Degrade gracefully to first upstream
        self.upstreams.first()
    }

    fn least_connections(&self) -> Option<&UpstreamRuntime> {
        // todo: no connection tracking yet, can't do least connections without them.
        // Degrade gracefully to round-robin
        self.round_robin()
    }
}

pub struct UpstreamRuntime {
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
                let (host, port) = parse_upstream(&u.url).expect("invalid upstream address");
                UpstreamRuntime {
                    host,
                    port,
                    use_tls: false,
                    sni: "".to_string(),
                }
            })
            .collect();

        out.insert(
            name.clone(),
            ServiceRuntime {
                strategy: svc.strategy.clone(),
                upstreams,
                round_robin_cursor: AtomicUsize::new(0),
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
fn parse_upstream(upstream_address: &str) -> Result<(String, u16)> {
    let mut parts = upstream_address.split(':');
    let host = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", upstream_address))?;
    let port = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", upstream_address))?
        .parse::<u16>()?;

    Ok((host.to_string(), port))
}
