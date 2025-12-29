use crate::conf::error::ConfigError;
use crate::conf::types::listener::ListenerConfig;
use crate::conf::types::{
    ParsedRoute, RouteConfig, RouteTarget, ServiceConfig, StaticCachePolicy, StaticFileConfig,
};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn validate_version(version: u32) -> Result<(), ConfigError> {
    if version != 1 {
        return Err(ConfigError::InvalidVersion { version });
    }

    Ok(())
}

pub fn validate_listeners(listeners: &[ListenerConfig]) -> Result<(), ConfigError> {
    let mut seen = std::collections::HashSet::new();

    for listener in listeners {
        // Validate address
        let addr: std::net::SocketAddr =
            listener
                .addr
                .parse()
                .map_err(|_| ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                })?;

        if !seen.insert(addr) {
            return Err(ConfigError::DuplicateListenerAddr {
                addr: listener.addr.clone(),
            });
        }

        if let Some(tls) = &listener.tls {
            let cert = PathBuf::from(&tls.cert);
            let key = PathBuf::from(&tls.key);

            if !cert.is_file() {
                return Err(ConfigError::MissingCertFile {
                    path: tls.cert.clone(),
                });
            }

            if !key.is_file() {
                return Err(ConfigError::MissingCertFile {
                    path: tls.key.clone(),
                });
            }
        }
    }

    Ok(())
}

pub fn validate_routes(
    routes: &[RouteConfig],
    services: &HashMap<String, ServiceConfig>,
) -> Result<(), ConfigError> {
    for route in routes {
        let RouteTarget::Service { name } = &route.target else {
            continue;
        };

        if !services.contains_key(name) {
            return Err(ConfigError::UnknownService {
                route: route.path.clone(),
                service: name.clone(),
            });
        }
    }

    // Validate services
    for (name, service) in services {
        if service.upstream.is_empty() {
            return Err(ConfigError::EmptyService {
                service: name.clone(),
            });
        }

        let cb = &service.circuit_breaker;
        if cb.enabled {
            if cb.failure_threshold == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "failure_threshold must be >= 1".to_string(),
                });
            }
            if cb.open_duration_ms == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "open_duration_ms must be >= 1".to_string(),
                });
            }
            if cb.half_open_max_requests == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "half_open_max_requests must be >= 1".to_string(),
                });
            }
            if cb.success_threshold == 0 {
                return Err(ConfigError::InvalidCircuitBreaker {
                    service: name.clone(),
                    reason: "success_threshold must be >= 1".to_string(),
                });
            }
        }
    }

    Ok(())
}

pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for parsed_route in routes {
        let target = match (parsed_route.service, parsed_route.file_dir) {
            (Some(service), None) => RouteTarget::Service { name: service },

            (None, Some(dir)) => {
                let static_config = StaticFileConfig::default();
                let cache_policy = StaticCachePolicy::default();
                RouteTarget::Static {
                    dir,
                    index: parsed_route.index,
                    directory_listing: parsed_route.directory_listing,
                    static_config,
                    cache_policy,
                }
            }

            _ => {
                return Err(ConfigError::InvalidRoute {
                    path: parsed_route.path,
                });
            }
        };

        if let (RouteTarget::Static { .. }, true) = (&target, parsed_route.allow_websocket) {
            return Err(ConfigError::InvalidRoute {
                path: parsed_route.path,
            });
        }

        out.push(RouteConfig {
            path: parsed_route.path,
            target,
            allow_websocket: parsed_route.allow_websocket,
            ws_idle_timeout_ms: parsed_route.ws_idle_timeout_ms,
            ws_max_connections: parsed_route.ws_max_connections,
        });
    }

    Ok(out)
}
