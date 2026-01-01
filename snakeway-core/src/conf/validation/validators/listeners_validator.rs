use crate::conf::types::listener::ListenerConfig;
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;

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
