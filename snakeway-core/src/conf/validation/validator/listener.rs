use crate::conf::types::ListenerConfig;
use crate::conf::validation::ConfigError;
use crate::conf::validation::ValidationCtx;
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
                ctx.error(ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                });
                continue;
            }
        };

        if !seen_addrs.insert(addr) {
            ctx.error(ConfigError::DuplicateListenerAddr {
                addr: listener.addr.clone(),
            });
        }

        if let Some(tls) = &listener.tls {
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

        if listener.enable_http2 && listener.tls.is_none() {
            ctx.error(ConfigError::Http2RequiresTls);
        }

        if listener.enable_admin {
            if admin_seen {
                ctx.error(ConfigError::MultipleAdminListeners);
            }
            admin_seen = true;

            if listener.enable_http2 {
                ctx.error(ConfigError::AdminListenerHttp2NotSupported);
            }
            if listener.tls.is_none() {
                ctx.error(ConfigError::AdminListenerMissingTls);
            }
            if addr.ip().is_unspecified() {
                ctx.error(ConfigError::InvalidListenerAddr {
                    addr: listener.addr.clone(),
                });
            }
        }
    }
}
