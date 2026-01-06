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

        for upstream in &service.tcp_upstreams {
            if upstream.weight == 0 {
                ctx.error(ConfigError::InvalidUpstream {
                    service: name.clone(),
                    upstream: upstream.url.clone(),
                    reason: "weight must be > 0".into(),
                });
            }

            let url = match upstream.url.parse::<url::Url>() {
                Ok(u) => u,
                Err(_) => {
                    ctx.error(ConfigError::InvalidUpstream {
                        service: name.clone(),
                        upstream: upstream.url.clone(),
                        reason: "invalid URL".into(),
                    });
                    continue;
                }
            };

            if !matches!(url.scheme(), "http" | "https") {
                ctx.error(ConfigError::InvalidUpstream {
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
            ctx.error(ConfigError::InvalidCircuitBreaker {
                service: name.clone(),
                reason: "all circuit breaker thresholds must be >= 1".into(),
            });
        }
    }
}
