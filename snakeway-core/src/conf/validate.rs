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

    Ok(())
}

pub fn compile_routes(routes: Vec<ParsedRoute>) -> Result<Vec<RouteConfig>, ConfigError> {
    let mut out = Vec::new();

    for r in routes {
        let target = match (r.service, r.file_dir) {
            (Some(service), None) => RouteTarget::Service { name: service },

            (None, Some(dir)) => {
                let static_config = StaticFileConfig::default();
                let cache_policy = StaticCachePolicy::default();
                RouteTarget::Static {
                    dir,
                    index: r.index,
                    directory_listing: r.directory_listing,
                    static_config,
                    cache_policy,
                }
            }

            _ => {
                return Err(ConfigError::InvalidRoute { path: r.path });
            }
        };

        out.push(RouteConfig {
            path: r.path,
            target,
        });
    }

    Ok(out)
}
