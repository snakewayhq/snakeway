use crate::conf::types::ServiceConfig;
use crate::conf::validation::ConfigError;
use crate::conf::validation::ValidationCtx;
use std::collections::HashMap;

/// Validate service definitions.
pub fn validate_services(services: &HashMap<String, ServiceConfig>, ctx: &mut ValidationCtx) {
    for (name, service) in services {
        if service.tcp_upstreams.is_empty() && service.unix_upstreams.is_empty() {
            ctx.error(ConfigError::EmptyService {
                service: name.clone(),
            });
            continue;
        }

        for tcp in &service.tcp_upstreams {
            if tcp.weight == 0 {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: tcp.url.clone(),
                    reason: "weight must be > 0".into(),
                });
            }

            let url = match tcp.url.parse::<url::Url>() {
                Ok(u) => u,
                Err(_) => {
                    ctx.error(ConfigError::InvalidUpstream {
                        service: name.clone(),
                        upstream: tcp.url.clone(),
                        reason: "invalid URL".into(),
                    });
                    continue;
                }
            };

            if !matches!(url.scheme(), "http" | "https") {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: tcp.url.clone(),
                    reason: "unsupported URL scheme".into(),
                });
            }
        }

        let mut seen_sock_values = HashMap::new();
        for unix in &service.unix_upstreams {
            if unix.weight == 0 {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: unix.sock.clone(),
                    reason: "weight must be > 0".into(),
                });
            }
            if seen_sock_values.contains_key(&unix.sock) {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: unix.sock.clone(),
                    reason: "duplicate socket path".into(),
                });
            } else {
                seen_sock_values.insert(unix.sock.clone(), ());
            }

            if unix.use_tls && unix.sni.is_empty() {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: unix.sock.clone(),
                    reason: "SNI must be set when using TLS".into(),
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
            ctx.error(ConfigError::InvalidCircuitBreaker {
                service: name.clone(),
                reason: "all circuit breaker thresholds must be >= 1".into(),
            });
        }
    }
}
