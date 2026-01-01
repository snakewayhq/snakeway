use crate::conf::error::ConfigError;
use crate::conf::types::listener::ListenerConfig;
use crate::conf::types::{
    LoadBalancingStrategy, ParsedRoute, ParsedRouteType, RouteConfig, RouteKind, ServiceConfig,
    StaticCachePolicy, StaticFileConfig,
};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Validate top-level config version.
pub fn validate_version(version: u32) -> Result<(), ConfigError> {
    if version == 1 {
        Ok(())
    } else {
        Err(ConfigError::InvalidVersion { version })
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

        if let Some(tls) = &listener.tls {
            if !Path::new(&tls.cert).is_file() {
                return Err(ConfigError::MissingCertFile {
                    path: tls.cert.clone(),
                });
            }
            if !Path::new(&tls.key).is_file() {
                return Err(ConfigError::MissingKeyFile {
                    path: tls.key.clone(),
                });
            }
        }

        if listener.enable_http2 && listener.tls.is_none() {
            return Err(ConfigError::Http2RequiresTls);
        }

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

        match &route.kind {
            RouteKind::Service { name } => {
                if !services.contains_key(name) {
                    return Err(ConfigError::UnknownService {
                        route: route.path.clone(),
                        service: name.clone(),
                    });
                }
            }

            RouteKind::Static { dir, .. } => {
                if route.allow_websocket {
                    return Err(ConfigError::WebSocketNotAllowedOnStaticRoute {
                        path: route.path.clone(),
                    });
                }

                let dir_path = Path::new(dir);
                if dir_path.is_relative() || !dir_path.is_dir() || dir_path == Path::new("/") {
                    return Err(ConfigError::InvalidStaticDir {
                        path: PathBuf::from(dir),
                        reason: "static routes must point to an absolute, non-root directory"
                            .into(),
                    });
                }
            }
        }
    }

    validate_services(services)
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

            if !matches!(url.scheme(), "http" | "https") {
                return Err(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: upstream.url.clone(),
                    reason: "unsupported URL scheme".into(),
                });
            }
        }

        let cb = &service.circuit_breaker;
        if cb.enable_auto_recovery
            && (cb.failure_threshold == 0
                || cb.open_duration_ms == 0
                || cb.half_open_max_requests == 0
                || cb.success_threshold == 0)
        {
            return Err(ConfigError::InvalidCircuitBreaker {
                service: name.clone(),
                reason: "all circuit breaker thresholds must be >= 1".into(),
            });
        }
    }

    Ok(())
}

/// Compile parsed routes into finalized RouteConfig values.
pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for parsed in routes {
        let path = match parsed.path.trim_end_matches('/') {
            "" => "/".to_string(),
            p => p.to_string(),
        };

        let kind = match parsed.r#type {
            ParsedRouteType::Service => {
                let service =
                    parsed
                        .service
                        .ok_or_else(|| ConfigError::MissingServiceForServiceRoute {
                            path: path.clone(),
                        })?;

                if parsed.file_dir.is_some() {
                    return Err(ConfigError::DirNotAllowedOnServiceRoute { path });
                }

                RouteKind::Service { name: service }
            }

            ParsedRouteType::Static => {
                let dir = parsed
                    .file_dir
                    .ok_or_else(|| ConfigError::MissingDirForStaticRoute { path: path.clone() })?;

                if parsed.service.is_some() {
                    return Err(ConfigError::ServiceNotAllowedOnStaticRoute { path });
                }

                if parsed.allow_websocket {
                    return Err(ConfigError::WebSocketNotAllowedOnStaticRoute { path });
                }

                RouteKind::Static {
                    dir,
                    index: parsed.index,
                    directory_listing: parsed.directory_listing,
                    static_config: StaticFileConfig::default(),
                    cache_policy: StaticCachePolicy::default(),
                }
            }
        };

        out.push(RouteConfig {
            path,
            kind,
            allow_websocket: parsed.allow_websocket,
            ws_idle_timeout_ms: parsed.ws_idle_timeout_ms,
            ws_max_connections: parsed.ws_max_connections,
        });
    }

    Ok(out)
}
