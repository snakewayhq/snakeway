use crate::control_plane::acme::{CertManager, SniRegistry};
use crate::execution::device::core::DeviceRegistry;
use crate::execution::route::types::RouteId;
use crate::execution::route::{RouteRuntime, Router};
use crate::runtime::error::ReloadError;
use crate::runtime::types::{
    ResolvedAddr, TlsRuntime, UpstreamAddr, UpstreamTcpRuntime, UpstreamUnixRuntime,
};
use crate::runtime::{RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime};
use ahash::RandomState;
use anyhow::{Context, Result, anyhow};
use arc_swap::ArcSwap;
use http::Uri;
use openssl::x509::X509;
use pingora::protocols::tls::CaType;
use snakeway_conf::types::{RouteConfig, ServiceConfig, UpstreamTcpConfig, UpstreamUnixConfig};
use snakeway_conf::{load_config, types::RuntimeConfig};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub(crate) async fn reload_runtime_state(
    config_path: &Path,
    state: &ArcSwap<RuntimeState>,
    cert_manager: &Option<Arc<CertManager>>,
) -> Result<RuntimeConfig, ReloadError> {
    let validated = load_config(config_path)?;

    if validated.has_warnings() {
        let mut out = String::new();
        validated.render_plain(&mut out);
        tracing::warn!("{out}");
    }

    let config = validated.config;
    let new_state = build_runtime_state(&config, cert_manager)?;

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

    // Attach the cert manager to the new SniRegistry BEFORE making the new
    // state live. This closes the window where a freshly issued cert could be
    // published into the old registry while handshakes already read from the
    // new one.
    if let (Some(manager), Some(tls)) = (cert_manager.as_ref(), new_state.tls.as_ref()) {
        manager.attach_tls_sni_map(tls.sni_map.clone());
    }

    // Atomic swap (point of no return).
    state.store(Arc::new(new_state));

    Ok(config)
}

/// Constructs the complete runtime state from configuration.
///
/// It takes the validated configuration and builds all the runtime components
/// needed to run the proxy: TLS certificate mappings, HTTP routers for request matching,
/// device registry, and service definitions with their upstream backends.
///
/// The resulting RuntimeState is immutable and thread-safe, designed to be swapped atomically
/// during configuration reloads without disrupting active connections.
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
    let global_ca_file = cfg.server.ca_file.as_deref().map(Path::new);
    let services = build_runtime_services(&cfg.services, global_ca_file)?;

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

    let registry = Arc::new(SniRegistry::new(sni_map));

    Ok(TlsRuntime { sni_map: registry })
}

/// Build service runtimes from config services.
/// The output is a map of service names to their respective runtimes.
fn build_runtime_services(
    services: &HashMap<String, ServiceConfig>,
    global_ca_file: Option<&Path>,
) -> Result<HashMap<String, ServiceRuntime>> {
    let mut out = HashMap::new();

    for (name, svc) in services {
        let mut upstreams = svc
            .tcp_upstreams
            .iter()
            .map(|u| {
                let rt = make_upstream_runtime_from_tcp(u, global_ca_file)?;
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
pub(crate) fn build_runtime_routers(routes: &[RouteConfig]) -> Result<HashMap<Arc<str>, Router>> {
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
fn make_upstream_runtime_from_tcp(
    cfg: &UpstreamTcpConfig,
    global_ca_file: Option<&Path>,
) -> Result<UpstreamRuntime> {
    let uri: Uri = cfg
        .url
        .parse()
        .map_err(|_| anyhow!("invalid upstream URL: {}", cfg.url))?;

    let authority = uri
        .authority()
        .ok_or_else(|| anyhow!("upstream URL missing authority: {}", cfg.url))?;

    let host = authority.host().to_string();

    let port = authority.port_u16().unwrap_or(80);

    // Resolve DNS eagerly so the data-plane hot path never calls getaddrinfo.
    // IP literals are parsed directly; hostnames are resolved via the OS resolver.
    let resolved_addr = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        std::net::SocketAddr::new(ip, port)
    } else {
        use std::net::ToSocketAddrs;
        (host.as_str(), port)
            .to_socket_addrs()
            .with_context(|| format!("failed to resolve upstream hostname '{host}'"))?
            .next()
            .ok_or_else(|| anyhow!("upstream hostname '{host}' resolved to no addresses"))?
    };

    let addr = UpstreamAddr::Tcp {
        host: host.clone(),
        port,
    };

    // Handle per-endpoint TLS settings.
    let use_tls = cfg.tls.is_some();
    let (verify, ca, group_key) = if let Some(tls_cfg) = &cfg.tls
        && tls_cfg.verify
    {
        // Prefer the per-endpoint ca_file; fall back to the global server.ca_file.
        let effective_ca = tls_cfg.ca_file.as_deref().or(global_ca_file);
        if let Some(ca_file) = effective_ca {
            let ca = load_ca_from_path(ca_file)?;
            let group_key = calculate_group_key(ca_file);
            (true, Some(Arc::new(ca)), group_key)
        } else {
            (false, None, 0)
        }
    } else {
        (false, None, 0)
    };

    // Determine SNI.
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
        resolved_addr: ResolvedAddr::new(resolved_addr),
        use_tls,
        sni,
        weight: cfg.weight,
        verify,
        ca,
        group_key,
    }))
}

/// Load a per-upstream CA file.
/// This happens when the runtime state is recomputed,
/// keeping it out of the data plane.
pub(crate) fn load_ca_from_path(path: &Path) -> Result<CaType> {
    if !path.exists() {
        anyhow::bail!("CA file does not exist: {}", path.display());
    }
    if !path.is_file() {
        anyhow::bail!("CA path is not a file: {}", path.display());
    }

    let pem =
        fs::read(path).with_context(|| format!("failed to read CA file: {}", path.display()))?;
    if pem.is_empty() {
        anyhow::bail!("CA file is empty: {}", path.display());
    }

    // Parse ALL certs in the PEM bundle.
    // stack_from_pem returns Vec<X509> (OpenSSL) / equivalent for boringssl shim.
    let certs = X509::stack_from_pem(&pem).with_context(|| {
        format!(
            "failed to parse PEM certificates in CA file: {}",
            path.display()
        )
    })?;

    if certs.is_empty() {
        anyhow::bail!(
            "CA file contained no certificates (parsed 0 certs): {}",
            path.display()
        );
    }

    Ok(certs.into_boxed_slice())
}

/// Hash a path to a u64.
/// This is used to group per-upstream CAs,
/// keeping them out of the data plane.
fn calculate_group_key(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = ahash::AHasher::default();
    path.as_os_str().hash(&mut h);
    h.finish()
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

/// Fixed-seed ahash - fast and deterministic across restarts.
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
