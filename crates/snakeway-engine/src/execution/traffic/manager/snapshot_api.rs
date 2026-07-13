use crate::execution::traffic::circuit::CircuitBreakerParams;
use crate::execution::traffic::{HealthCheckParams, ServiceId, TrafficManager, TrafficSnapshot};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// Snapshot API (read-only)
impl TrafficManager {
    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
        let valid_services: HashSet<ServiceId> = new_snapshot.services.keys().cloned().collect();

        // Clean up weighted round-robin cursors
        self.wrr_state
            .retain(|service_id, _| valid_services.contains(service_id));

        // Cleanup active request counters
        self.active_requests.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });

        // Cleanup health state
        self.upstream_health.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });

        // Cleanup total counters
        self.total_requests.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });
        self.total_successes.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });
        self.total_failures.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });

        // Cleanup circuit breaker state
        self.circuit.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| {
                    svc.upstreams
                        .iter()
                        .any(|u| u.endpoint.id() == *upstream_id)
                })
                .unwrap_or(false)
        });

        // Cleanup circuit breaker parameters
        self.circuit_params
            .retain(|service_id, _| valid_services.contains(service_id));

        // Cleanup health check parameters
        self.health_params
            .retain(|service_id, _| valid_services.contains(service_id));

        for (svc_id, svc) in new_snapshot.services.iter() {
            // Clone circuit breaker params...
            let params = CircuitBreakerParams {
                enable_auto_recovery: svc.circuit_breaker_cfg.enable_auto_recovery,
                failure_threshold: svc.circuit_breaker_cfg.failure_threshold,
                open_duration: Duration::from_millis(
                    svc.circuit_breaker_cfg.open_duration_milliseconds,
                ),
                half_open_max_requests: svc.circuit_breaker_cfg.half_open_max_requests,
                success_threshold: svc.circuit_breaker_cfg.success_threshold,
                count_http_5xx_as_failure: svc.circuit_breaker_cfg.count_http_5xx_as_failure,
            };
            self.circuit_params.insert(svc_id.clone(), Arc::new(params));

            // And, clone health check params...
            let health_params = HealthCheckParams {
                enable: svc.health_check_cfg.enable,
                failure_threshold: svc.health_check_cfg.failure_threshold,
                unhealthy_cooldown: Duration::from_secs(
                    svc.health_check_cfg.unhealthy_cooldown_seconds,
                ),
            };

            self.health_params
                .insert(svc_id.clone(), Arc::new(health_params));
        }

        self.snapshot.store(Arc::new(new_snapshot));
    }
}
