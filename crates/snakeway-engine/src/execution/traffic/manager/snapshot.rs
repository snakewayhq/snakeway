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

        // Cleanup latency measurements
        self.latency_stats.retain(|(service_id, upstream_id), _| {
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
                enable: svc.circuit_breaker_cfg.enable,
                failure_threshold: svc.circuit_breaker_cfg.failure_threshold,
                open_duration: Duration::from_millis(
                    svc.circuit_breaker_cfg.open_duration_milliseconds,
                ),
                half_open_max_requests: svc.circuit_breaker_cfg.half_open_max_requests,
                success_threshold: svc.circuit_breaker_cfg.success_threshold,
                count_http_5xx_as_failure: svc.circuit_breaker_cfg.count_http_5xx_as_failure,
            };
            // State kept from before the circuit breaker was turned on or off could leave an
            // upstream stuck open or half open, so every circuit of the service starts closed.
            let enable_changed = self
                .circuit_params
                .get(svc_id)
                .is_some_and(|old| old.enable != params.enable);
            if enable_changed {
                self.circuit
                    .retain(|(service_id, _), _| service_id != svc_id);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::traffic::ServiceSnapshot;
    use crate::execution::traffic::UpstreamSnapshot;
    use crate::execution::traffic::circuit::CircuitState;
    use crate::execution::traffic::snapshot::{
        service_metric_attribute, upstream_metric_attributes,
    };
    use crate::runtime::{ResolvedAddr, UpstreamId, UpstreamRuntime, UpstreamTcpRuntime};
    use snakeway_conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
    use std::collections::HashMap;

    fn service() -> ServiceId {
        ServiceId("api".into())
    }

    fn upstream() -> UpstreamId {
        UpstreamId::new(1)
    }

    fn snapshot(enable: bool, open_duration_milliseconds: u64) -> TrafficSnapshot {
        let name: Arc<str> = Arc::from("api");
        let upstream = UpstreamSnapshot {
            endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                id: upstream(),
                host: "127.0.0.1".to_string(),
                port: 8001,
                resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], 8001).into()),
                tls: None,
                weight: 1,
            }),
            weight: 1,
            metric_attributes: upstream_metric_attributes(&name, upstream()),
        };
        let mut services = HashMap::new();
        services.insert(
            service(),
            ServiceSnapshot {
                service_id: service(),
                strategy: LoadBalancingStrategy::Failover,
                upstreams: vec![upstream],
                circuit_breaker_cfg: CircuitBreakerConfig {
                    enable,
                    failure_threshold: 1,
                    open_duration_milliseconds,
                    half_open_max_requests: 1,
                    success_threshold: 1,
                    count_http_5xx_as_failure: true,
                },
                health_check_cfg: HealthCheckConfig::default(),
                metric_service: service_metric_attribute(&name),
            },
        );
        TrafficSnapshot { services }
    }

    fn trip_open(manager: &TrafficManager) {
        assert!(manager.circuit_allows(&service(), &upstream()));
        manager.circuit_on_end(&service(), &upstream(), true, false);
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::Open)
        );
    }

    #[test]
    fn a_reload_that_turns_the_circuit_breaker_off_releases_an_open_circuit() {
        // Arrange
        let manager = TrafficManager::new(snapshot(true, 60_000));
        trip_open(&manager);
        manager.update(snapshot(false, 60_000));

        // Act
        let allowed = manager.circuit_allows(&service(), &upstream());

        // Assert
        assert!(allowed);
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::Closed)
        );
    }

    #[test]
    fn a_reload_that_turns_the_circuit_breaker_off_releases_a_half_open_circuit() {
        // Arrange
        let manager = TrafficManager::new(snapshot(true, 0));
        trip_open(&manager);
        assert!(manager.circuit_allows(&service(), &upstream()));
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::HalfOpen)
        );
        manager.update(snapshot(false, 0));

        // Act
        let allowed = manager.circuit_allows(&service(), &upstream());

        // Assert
        assert!(allowed);
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::Closed)
        );
    }

    #[test]
    fn a_reload_that_turns_the_circuit_breaker_back_on_starts_with_a_closed_circuit() {
        // Arrange
        let manager = TrafficManager::new(snapshot(true, 0));
        trip_open(&manager);
        assert!(manager.circuit_allows(&service(), &upstream()));
        manager.update(snapshot(false, 0));
        manager.circuit_on_end(&service(), &upstream(), true, true);
        manager.update(snapshot(true, 0));

        // Act
        let allowed = manager.circuit_allows(&service(), &upstream());

        // Assert
        assert!(allowed);
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::Closed)
        );
    }

    #[test]
    fn a_reload_that_keeps_the_circuit_breaker_on_keeps_an_open_circuit() {
        // Arrange
        let manager = TrafficManager::new(snapshot(true, 60_000));
        trip_open(&manager);
        manager.update(snapshot(true, 60_000));

        // Act
        let allowed = manager.circuit_allows(&service(), &upstream());

        // Assert
        assert!(!allowed);
        assert_eq!(
            manager.circuit_state(&service(), &upstream()),
            Some(CircuitState::Open)
        );
    }
}
