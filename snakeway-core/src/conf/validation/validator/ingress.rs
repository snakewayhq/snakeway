use crate::conf::types::IngressConfig;
use crate::conf::validation::ConfigError;
use crate::conf::validation::ValidationCtx;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub fn validate_ingresses(ingresses: &[IngressConfig], ctx: &mut ValidationCtx) {
    let mut seen_listener_addrs = HashSet::new();

    for ingress in ingresses {
        if let Some(bind) = &ingress.bind {
            let bind_addr: SocketAddr = match bind.addr.parse() {
                Ok(a) => a,
                Err(_) => {
                    ctx.error(ConfigError::InvalidListenerAddr {
                        addr: bind.addr.clone(),
                    });
                    continue;
                }
            };

            if bind_addr.ip().is_unspecified() {
                ctx.error(ConfigError::InvalidListenerAddr {
                    addr: bind_addr.to_string(),
                });
            }

            if !seen_listener_addrs.insert(&bind.addr) {
                ctx.error(ConfigError::DuplicateListenerAddr {
                    addr: bind.addr.clone(),
                })
            }

            if let Some(tls) = &bind.tls {
                if !Path::new(&tls.cert).is_file() {
                    ctx.error(ConfigError::MissingCertFile {
                        path: tls.cert.clone(),
                    });
                }
                if !Path::new(&tls.key).is_file() {
                    ctx.error(ConfigError::MissingKeyFile {
                        path: tls.key.clone(),
                    });
                }
            }

            // HTTP/2 requires TLS.
            if bind.enable_http2 && bind.tls.is_none() {
                ctx.error(ConfigError::Http2RequiresTls);
            }
        }

        if let Some(bind_admin) = &ingress.bind_admin {
            if !seen_listener_addrs.insert(&bind_admin.addr) {
                ctx.error(ConfigError::DuplicateListenerAddr {
                    addr: bind_admin.addr.clone(),
                })
            }

            let bind_admin_addr: SocketAddr = match bind_admin.addr.parse() {
                Ok(a) => a,
                Err(_) => {
                    ctx.error(ConfigError::InvalidListenerAddr {
                        addr: bind_admin.addr.clone(),
                    });
                    continue;
                }
            };

            if bind_admin_addr.ip().is_unspecified() {
                ctx.error(ConfigError::InvalidListenerAddr {
                    addr: bind_admin_addr.to_string(),
                });
            }
        }
    }
}
