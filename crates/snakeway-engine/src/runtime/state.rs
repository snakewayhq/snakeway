use crate::execution::device::core::DeviceRegistry;
use crate::execution::route::types::RouteId;
use crate::execution::route::{RouteRuntime, Router};
use crate::runtime::error::ReloadError;
use crate::runtime::manual_tls::load_manual_certs;
use crate::runtime::types::{
    ResolvedAddr, TlsRuntime, UpstreamAddr, UpstreamTcpRuntime, UpstreamUnixRuntime,
};
use crate::runtime::upstream_tls::resolve_upstream_tls;
use crate::runtime::{RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime};
use ahash::RandomState;
use anyhow::{Context, Result, anyhow};
use arc_swap::ArcSwap;
use http::Uri;
use snakeway_acme::{CertManager, SniRegistry};
use snakeway_conf::types::{RouteConfig, ServiceConfig, UpstreamTcpConfig, UpstreamUnixConfig};
use snakeway_conf::{load_config, types::RuntimeConfig};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub async fn reload_runtime_state(
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
    let manual_certs = load_manual_certs(&cfg.listeners)?;

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
        manual_certs,
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
                    let rt = make_upstream_runtime_for_unix(u, global_ca_file)?;
                    Ok(rt)
                })
                .collect::<Result<Vec<_>>>()?,
        );

        out.insert(
            name.clone(),
            ServiceRuntime {
                strategy: svc.load_balancing_strategy,
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
        weight: cfg.weight,
        tls: resolve_upstream_tls(cfg.tls.as_ref(), sni, global_ca_file)?,
    }))
}

/// Factory function to make a unix upstream runtime.
fn make_upstream_runtime_for_unix(
    cfg: &UpstreamUnixConfig,
    global_ca_file: Option<&Path>,
) -> Result<UpstreamRuntime> {
    let addr = UpstreamAddr::Unix {
        path: cfg.sock.clone(),
    };
    Ok(UpstreamRuntime::Unix(UpstreamUnixRuntime {
        id: make_upstream_id(&addr),
        path: cfg.sock.clone(),
        weight: cfg.weight,
        tls: resolve_upstream_tls(
            cfg.tls.as_ref(),
            cfg.tls.as_ref().map(|t| t.sni.clone()).unwrap_or_default(),
            global_ca_file,
        )?,
    }))
}

/// Fixed-seed ahash, so an upstream keeps the same id across restarts.
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

#[cfg(test)]
mod tests {
    use super::*;
    use snakeway_conf::types::UpstreamTlsConfig;

    fn unix_upstream_config(tls: Option<UpstreamTlsConfig>) -> UpstreamUnixConfig {
        UpstreamUnixConfig {
            sock: "/tmp/app.sock".to_string(),
            weight: 1,
            tls,
        }
    }

    #[test]
    fn unix_upstream_without_tls_block_uses_plain_http() {
        // Arrange
        let cfg = unix_upstream_config(None);

        // Act
        let result = make_upstream_runtime_for_unix(&cfg, None);

        // Assert
        let UpstreamRuntime::Unix(unix) = result.expect("the upstream runtime must build") else {
            panic!("expected a Unix upstream runtime");
        };
        assert!(unix.tls.is_none());
    }

    #[test]
    fn unix_upstream_with_verify_and_no_ca_file_keeps_verification_on() {
        // Arrange
        let cfg = unix_upstream_config(Some(UpstreamTlsConfig {
            sni: "app.internal".to_string(),
            verify: true,
            ca_file: None,
        }));

        // Act
        let result = make_upstream_runtime_for_unix(&cfg, None);

        // Assert
        let UpstreamRuntime::Unix(unix) = result.expect("the upstream runtime must build") else {
            panic!("expected a Unix upstream runtime");
        };
        let tls = unix.tls.expect("the upstream must use TLS");
        assert!(tls.verify);
        assert_eq!(tls.sni, "app.internal");
        assert!(tls.ca.is_none());
        assert_eq!(tls.group_key, 0);
    }

    fn tcp_upstream_config(verify: bool) -> UpstreamTcpConfig {
        UpstreamTcpConfig {
            url: "https://127.0.0.1:8443".to_string(),
            weight: 1,
            tls: Some(snakeway_conf::types::UpstreamTlsConfig {
                sni: "backend.test".to_string(),
                verify,
                ca_file: None,
            }),
        }
    }

    #[test]
    fn tcp_upstream_with_verify_and_no_ca_file_keeps_verification_on() {
        // Arrange
        let cfg = tcp_upstream_config(true);

        // Act
        let result = make_upstream_runtime_from_tcp(&cfg, None);

        // Assert
        let UpstreamRuntime::Tcp(tcp) = result.expect("the upstream runtime must build") else {
            panic!("expected a TCP upstream runtime");
        };
        let tls = tcp.tls.expect("the upstream must use TLS");
        assert!(tls.verify);
        assert_eq!(tls.sni, "backend.test");
        assert!(tls.ca.is_none());
        assert_eq!(tls.group_key, 0);
    }

    #[test]
    fn tcp_upstream_with_verify_false_skips_verification() {
        // Arrange
        let cfg = tcp_upstream_config(false);

        // Act
        let result = make_upstream_runtime_from_tcp(&cfg, None);

        // Assert
        let UpstreamRuntime::Tcp(tcp) = result.expect("the upstream runtime must build") else {
            panic!("expected a TCP upstream runtime");
        };
        let tls = tcp.tls.expect("the upstream must use TLS");
        assert!(!tls.verify);
        assert!(tls.ca.is_none());
    }
}
