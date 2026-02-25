use crate::cert_manager::CertManager;
use crate::conf::types::{RouteConfig, ServiceConfig, UpstreamTcpConfig, UpstreamUnixConfig};
use crate::conf::{RuntimeConfig, load_config};
use crate::device::core::registry::DeviceRegistry;
use crate::route::types::RouteId;
use crate::route::{RouteRuntime, Router};
use crate::runtime::error::ReloadError;
use crate::runtime::types::{TlsRuntime, UpstreamAddr, UpstreamTcpRuntime, UpstreamUnixRuntime};
use crate::runtime::{RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime};
use ahash::RandomState;
use anyhow::{Result, anyhow};
use arc_swap::ArcSwap;
use http::Uri;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub async fn reload_runtime_state(
    config_path: &Path,
    state: &ArcSwap<RuntimeState>,
    cert_manager: &Option<Arc<CertManager>>,
) -> Result<RuntimeConfig, ReloadError> {
    // Parse and validate config.
    let validated = load_config(config_path)?;

    if !validated.is_valid() {
        return Err(ReloadError::InvalidConfig {
            report: validated.validation_report,
        });
    }

    // Build a new runtime state OFFLINE.
    let new_state = build_runtime_state(&validated.config, cert_manager)?;

    // Log comparison against current state.
    let old = state.load();
    let old_routers = old.routers.len();
    tracing::info!(
        old_routers = old_routers,
        old_devices = old.devices.all().len(),
        new_routers = new_state.routers.len(),
        new_devices = new_state.devices.all().len(),
        "runtime state reloaded"
    );

    // Atomic swap (point of no return).
    state.store(Arc::new(new_state));

    Ok(validated.config)
}

pub fn build_runtime_state(
    cfg: &RuntimeConfig,
    cert_manager: &Option<Arc<CertManager>>,
) -> Result<RuntimeState> {
    // TLS Certificates
    let tls: Option<TlsRuntime> = cert_manager.as_ref().map(build_tls_runtime).transpose()?;

    // Routers
    let routers = build_runtime_routers(&cfg.routes)?;

    // Devices
    let mut devices = DeviceRegistry::new();
    devices.load_from_config(cfg)?;
    tracing::debug!("Loaded device count = {}", devices.all().len());

    // Services
    let services = build_runtime_services(&cfg.services)?;

    Ok(RuntimeState {
        tls,
        routers,
        devices,
        services,
    })
}

/// Build the TLS SNI -> Cert runtime map.
fn build_tls_runtime(cert_manager: &Arc<CertManager>) -> Result<TlsRuntime> {
    let sni_map = cert_manager.build_sni_map()?;
    Ok(TlsRuntime {
        sni_map: ArcSwap::new(Arc::new(sni_map)),
    })
}

/// Build service runtimes from config services.
/// The output is a map of service names to their respective runtimes.
fn build_runtime_services(
    services: &HashMap<String, ServiceConfig>,
) -> Result<HashMap<String, ServiceRuntime>> {
    let mut out = HashMap::new();

    for (name, svc) in services {
        let mut upstreams = svc
            .tcp_upstreams
            .iter()
            .map(|u| {
                let rt = make_upstream_runtime_from_tcp(u)?;
                Ok(rt)
            })
            .collect::<Result<Vec<_>>>()?;

        upstreams.extend(
            svc.unix_upstreams
                .iter()
                .map(|u| {
                    let rt = make_upstream_runtime_for_unix(u)?;
                    Ok(rt)
                })
                .collect::<Result<Vec<_>>>()?,
        );

        out.insert(
            name.clone(),
            ServiceRuntime {
                strategy: svc.load_balancing_strategy.clone(),
                upstreams,
                circuit_breaker_cfg: svc.circuit_breaker.clone(),
                health_check_cfg: svc.health_check.clone(),
                listener: Some(Arc::from(svc.listener.clone())),
            },
        );
    }

    Ok(out)
}

/// Build router from config routes.
pub fn build_runtime_routers(routes: &[RouteConfig]) -> Result<HashMap<Arc<str>, Router>> {
    let mut routers: HashMap<Arc<str>, Router> = HashMap::new();

    for route in routes {
        let listener = route.listener();

        let router = routers.entry(Arc::from(listener)).or_default();

        let route_runtime = match route {
            RouteConfig::Service(cfg) => RouteRuntime::Service {
                id: RouteId::service(&cfg.path, &cfg.service),
                upstream: cfg.service.clone(),
                allow_websocket: cfg.allow_websocket,
                ws_max_connections: cfg.ws_max_connections,
            },
            RouteConfig::Static(cfg) => RouteRuntime::Static {
                id: RouteId::static_route(&cfg.path, &canonicalize_dir(&cfg.file_dir)),
                path: cfg.path.clone(),
                file_dir: cfg.file_dir.clone(),
                index: cfg.index.is_some(),
                directory_listing: cfg.directory_listing,
                max_file_size: cfg.max_file_size,
                static_config: cfg.static_config.clone(),
                cache_policy: cfg.cache_policy.clone(),
            },
        };

        router.add_route(route.hosts(), route.path(), route_runtime)?;
    }

    Ok(routers)
}

/// Factory function to make a TCP upstream runtime.
fn make_upstream_runtime_from_tcp(cfg: &UpstreamTcpConfig) -> Result<UpstreamRuntime> {
    let uri: Uri = cfg
        .url
        .parse()
        .map_err(|_| anyhow!("invalid upstream URL: {}", cfg.url))?;

    let authority = uri
        .authority()
        .ok_or_else(|| anyhow!("upstream URL missing authority: {}", cfg.url))?;

    let host = authority.host().to_string();

    let port = authority.port_u16().unwrap_or(80);

    let addr = UpstreamAddr::Tcp {
        host: host.clone(),
        port,
    };

    // TLS is explicit now
    let use_tls = cfg.tls.is_some();

    // Determine SNI
    let sni = if let Some(tls_cfg) = &cfg.tls {
        // Explicit SNI overrides everything
        if !tls_cfg.sni.trim().is_empty() {
            tls_cfg.sni.clone()
        } else if host.parse::<std::net::IpAddr>().is_ok() {
            // If the host is an IP and there is no explicit SNI, do not send SNI.
            // This should be impossible because the conf system should have validated it before
            // the runtime config is created.
            String::new()
        } else {
            // Host is DNS, this the safe default if TLS is enabled and no explicit SNI is set.
            host.clone()
        }
    } else {
        // No TLS, then no SNI.
        String::new()
    };

    Ok(UpstreamRuntime::Tcp(UpstreamTcpRuntime {
        id: make_upstream_id(&addr),
        host,
        port,
        use_tls,
        sni,
        weight: cfg.weight,
    }))
}
/// Factory function to make a unix upstream runtime.
fn make_upstream_runtime_for_unix(cfg: &UpstreamUnixConfig) -> Result<UpstreamRuntime> {
    let addr = UpstreamAddr::Unix {
        path: cfg.sock.clone(),
    };
    Ok(UpstreamRuntime::Unix(UpstreamUnixRuntime {
        id: make_upstream_id(&addr),
        path: cfg.sock.clone(),
        use_tls: cfg.use_tls,
        sni: cfg.sni.clone(),
        weight: cfg.weight,
    }))
}

// Fixed-seed ahash:
// - deterministic across restarts
// - fast
// - not used for security
fn make_upstream_id(addr: &UpstreamAddr) -> UpstreamId {
    static HASHER: RandomState = RandomState::with_seeds(1, 2, 3, 4);

    UpstreamId(HASHER.hash_one(addr) as u32)
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
