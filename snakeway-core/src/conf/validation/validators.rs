use crate::conf::types::listener::ListenerConfig;
use crate::conf::types::{
    EntrypointConfig, ParsedRoute, ParsedRouteType, RouteConfig, RouteKind, ServiceConfig,
    StaticCachePolicy, StaticFileConfig,
};
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::{ValidationCtx, ValidationErrors};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Validate everything that exists in a fully parsed config.
pub fn validate_runtime_config(
    entry: &EntrypointConfig,
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
) -> Result<(), ValidationErrors> {
    let mut ctx = ValidationCtx::default();

    if let Err(e) = validate_version(entry.server.version) {
        ctx.push(e);
    }

    validate_listeners(&entry.listeners, &mut ctx);
    validate_routes(routes, services, &mut ctx);
    validate_services(services, &mut ctx);

    ctx.into_result()
}

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub fn validate_version(version: u32) -> Result<(), ConfigError> {
    if version == 1 {
        Ok(())
    } else {
        Err(ConfigError::InvalidVersion { version })
    }
}

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub fn validate_listeners(listeners: &[ListenerConfig], ctx: &mut ValidationCtx) {
    let mut seen_addrs = HashSet::new();
    let mut admin_seen = false;

    for listener in listeners {
        let addr: SocketAddr = match listener.addr.parse() {
            Ok(a) => a,
            Err(_) => {
                ctx.push(ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                });
                continue;
            }
        };

        if !seen_addrs.insert(addr) {
            ctx.push(ConfigError::DuplicateListenerAddr {
                addr: listener.addr.clone(),
            });
        }

        if let Some(tls) = &listener.tls {
            if !Path::new(&tls.cert).is_file() {
                ctx.push(ConfigError::MissingCertFile {
                    path: tls.cert.clone(),
                });
            }
            if !Path::new(&tls.key).is_file() {
                ctx.push(ConfigError::MissingKeyFile {
                    path: tls.key.clone(),
                });
            }
        }

        if listener.enable_http2 && listener.tls.is_none() {
            ctx.push(ConfigError::Http2RequiresTls);
        }

        if listener.enable_admin {
            if admin_seen {
                ctx.push(ConfigError::MultipleAdminListeners);
            }
            admin_seen = true;

            if listener.enable_http2 {
                ctx.push(ConfigError::AdminListenerHttp2NotSupported);
            }
            if listener.tls.is_none() {
                ctx.push(ConfigError::AdminListenerMissingTls);
            }
            if addr.ip().is_unspecified() {
                ctx.push(ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                });
            }
        }
    }
}

/// Validate routes and referenced services.
pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
    ctx: &mut ValidationCtx,
) {
    let mut seen_paths = HashSet::new();

    for route in routes {
        if !route.path.starts_with('/') {
            ctx.push(ConfigError::InvalidRoutePath {
                path: route.path.clone(),
                reason: "route path must start with '/'".into(),
            });
        }

        if !seen_paths.insert(route.path.clone()) {
            ctx.push(ConfigError::DuplicateRoute {
                path: route.path.clone(),
            });
        }

        match &route.kind {
            RouteKind::Service { name } => {
                if !services.contains_key(name) {
                    ctx.push(ConfigError::UnknownService {
                        route: route.path.clone(),
                        service: name.clone(),
                    });
                }
            }

            RouteKind::Static { dir, .. } => {
                if route.allow_websocket {
                    ctx.push(ConfigError::WebSocketNotAllowedOnStaticRoute {
                        path: route.path.clone(),
                    });
                }

                let dir_path = Path::new(dir);
                if dir_path.is_relative() || !dir_path.is_dir() || dir_path == Path::new("/") {
                    ctx.push(ConfigError::InvalidStaticDir {
                        path: PathBuf::from(dir),
                        reason: "static routes must point to an absolute, non-root directory"
                            .into(),
                    });
                }
            }
        }
    }
}

/// Validate service definitions.
pub fn validate_services(services: &HashMap<String, ServiceConfig>, ctx: &mut ValidationCtx) {
    for (name, service) in services {
        if service.upstream.is_empty() {
            ctx.push(ConfigError::EmptyService {
                service: name.clone(),
            });
            continue;
        }

        for upstream in &service.upstream {
            if upstream.weight == 0 {
                ctx.push(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: upstream.url.clone(),
                    reason: "weight must be > 0".into(),
                });
            }

            let url = match upstream.url.parse::<url::Url>() {
                Ok(u) => u,
                Err(_) => {
                    ctx.push(ConfigError::InvalidUpstream {
                        service: name.clone(),
                        upstream: upstream.url.clone(),
                        reason: "invalid URL".into(),
                    });
                    continue;
                }
            };

            if !matches!(url.scheme(), "http" | "https") {
                ctx.push(ConfigError::InvalidUpstream {
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
            ctx.push(ConfigError::InvalidCircuitBreaker {
                service: name.clone(),
                reason: "all circuit breaker thresholds must be >= 1".into(),
            });
        }
    }
}

/// Compile parsed routes into finalized RouteConfig values.
///
/// Still fail-fast: malformed route declarations cannot be meaningfully validated.
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
