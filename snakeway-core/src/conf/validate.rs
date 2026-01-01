use crate::conf::error::ConfigError;
use crate::conf::types::listener::ListenerConfig;
use crate::conf::types::{
    ParsedRoute, RouteConfig, RouteTarget, ServiceConfig, StaticCachePolicy, StaticFileConfig,
};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Validate top-level config version.
pub fn validate_version(version: u32) -> Result<(), ConfigError> {
    match version {
        1 => Ok(()),
        _ => Err(ConfigError::InvalidVersion { version }),
    }
}

/// Validate listener definitions.
pub fn validate_listeners(listeners: &[ListenerConfig]) -> Result<(), ConfigError> {
    let mut seen_addrs = HashSet::new();
    let mut admin_seen = false;

    for listener in listeners {
        let addr: SocketAddr =
            listener
                .addr
                .parse()
                .map_err(|_| ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                })?;

        if !seen_addrs.insert(addr) {
            return Err(ConfigError::DuplicateListenerAddr {
                addr: listener.addr.clone(),
            });
        }

        // TLS validation
        if let Some(tls) = &listener.tls {
            let cert = Path::new(&tls.cert);
            let key = Path::new(&tls.key);

            if !cert.is_file() {
                return Err(ConfigError::MissingCertFile {
                    path: tls.cert.clone(),
                });
            }

            if !key.is_file() {
                return Err(ConfigError::MissingKeyFile {
                    path: tls.key.clone(),
                });
            }
        }

        // HTTP/2 requires TLS
        if listener.enable_http2 && listener.tls.is_none() {
            return Err(ConfigError::Http2RequiresTls);
        }

        // Admin listener invariants
        if listener.enable_admin {
            if admin_seen {
                return Err(ConfigError::MultipleAdminListeners);
            }
            admin_seen = true;

            if listener.enable_http2 {
                return Err(ConfigError::AdminListenerHttp2NotSupported);
            }

            if listener.tls.is_none() {
                return Err(ConfigError::AdminListenerMissingTls);
            }

            // Do not allow admin listeners on wildcard addresses
            if addr.ip().is_unspecified() {
                return Err(ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validate routes and referenced services.
pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
) -> Result<(), ConfigError> {
    let mut seen_paths = HashSet::new();

    for route in routes {
        // Path must be absolute
        if !route.path.starts_with('/') {
            return Err(ConfigError::InvalidRoutePath {
                path: route.path.clone(),
                reason: "route path must start with '/'".into(),
            });
        }

        if !seen_paths.insert(route.path.clone()) {
            return Err(ConfigError::DuplicateRoute {
                path: route.path.clone(),
            });
        }

        match &route.target {
            RouteTarget::Service { name } => {
                if !services.contains_key(name) {
                    return Err(ConfigError::UnknownService {
                        route: route.path.clone(),
                        service: name.clone(),
                    });
                }
            }

            RouteTarget::Static { dir, .. } => {
                if route.allow_websocket {
                    return Err(ConfigError::InvalidRoute {
                        path: route.path.clone(),
                    });
                }

                let path = Path::new(dir);

                // Static paths must be absolute and real directories
                if path.is_relative() || !path.is_dir() || path == Path::new("/") {
                    return Err(ConfigError::InvalidStaticDir {
                        path: PathBuf::from(dir),
                        reason: "Static paths must be absolute and real directories".to_string(),
                    });
                }
            }
        }

        // WebSocket routes must target services
        if route.allow_websocket && !matches!(route.target, RouteTarget::Service { .. }) {
            return Err(ConfigError::InvalidRoute {
                path: route.path.clone(),
            });
        }
    }

    validate_services(services)?;

    Ok(())
}

/// Validate service definitions.
fn validate_services(services: &HashMap<String, ServiceConfig>) -> Result<(), ConfigError> {
    for (name, service) in services {
        if service.upstream.is_empty() {
            return Err(ConfigError::EmptyService {
                service: name.clone(),
            });
        }

        for upstream in &service.upstream {
            if upstream.weight == 0 {
                return Err(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: upstream.url.clone(),
                    reason: "weight must be > 0".into(),
                });
            }

            let url =
                upstream
                    .url
                    .parse::<url::Url>()
                    .map_err(|_| ConfigError::InvalidUpstream {
                        service: name.clone(),
                        upstream: upstream.url.clone(),
                        reason: "invalid URL".into(),
                    })?;

            if url.scheme() != "http" && url.scheme() != "https" {
                return Err(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: upstream.url.clone(),
                    reason: "unsupported URL scheme".into(),
                });
            }
        }

        let cb = &service.circuit_breaker;
        if cb.enable_auto_recovery {
            if cb.failure_threshold == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "failure_threshold must be >= 1".into(),
                });
            }

            if cb.open_duration_ms == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "open_duration_ms must be >= 1".into(),
                });
            }

            if cb.half_open_max_requests == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "half_open_max_requests must be >= 1".into(),
                });
            }

            if cb.success_threshold == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "success_threshold must be >= 1".into(),
                });
            }
        }
    }

    Ok(())
}

/// Compile parsed routes into finalized RouteConfig values.
pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for parsed in routes {
        let path = if parsed.path.ends_with('/') && parsed.path != "/" {
            parsed.path.trim_end_matches('/').to_string()
        } else {
            parsed.path
        };

        let target = match (parsed.service, parsed.file_dir) {
            (Some(service), None) => RouteTarget::Service { name: service },

            (None, Some(dir)) => RouteTarget::Static {
                dir,
                index: parsed.index,
                directory_listing: parsed.directory_listing,
                static_config: StaticFileConfig::default(),
                cache_policy: StaticCachePolicy::default(),
            },

            _ => {
                return Err(ConfigError::InvalidRoute { path });
            }
        };

        if matches!(target, RouteTarget::Static { .. }) && parsed.allow_websocket {
            return Err(ConfigError::InvalidRoute { path });
        }

        out.push(RouteConfig {
            path,
            target,
            allow_websocket: parsed.allow_websocket,
            ws_idle_timeout_ms: parsed.ws_idle_timeout_ms,
            ws_max_connections: parsed.ws_max_connections,
        });
    }

    Ok(out)
}
