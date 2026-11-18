use crate::execution::traffic::circuit::CircuitState;
use crate::execution::traffic::{
    HealthState, HealthStatus, LatencyStats, ServiceId, TrafficManager,
};
use crate::runtime::UpstreamId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Health API
impl TrafficManager {
    pub(crate) fn report_failure(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        // A reload can remove the service while one of its requests is still in flight.
        let Some(health_params) = self.health_params.get(service_id) else {
            return;
        };

        let key = (service_id.clone(), *upstream_id);

        let total = self
            .total_failures
            .entry(key.clone())
            .or_insert_with(|| AtomicU64::new(0));
        total.fetch_add(1, Ordering::Relaxed);

        if !health_params.enable {
            // Health checks are disabled for this service,
            // so we short-circuit updating health status after reporting total failures.
            return;
        }

        let mut entry = self
            .upstream_health
            .entry(key)
            .or_insert_with(|| HealthState::Healthy);

        *entry = match *entry {
            // First failure
            HealthState::Healthy => HealthState::Unhealthy {
                consecutive_failures: 1,
                last_failure: Instant::now(),
            },

            // Below threshold, then increment only
            HealthState::Unhealthy {
                consecutive_failures,
                ..
            } if consecutive_failures + 1 < health_params.failure_threshold => {
                HealthState::Unhealthy {
                    consecutive_failures: consecutive_failures + 1,
                    last_failure: Instant::now(),
                }
            }

            // Threshold reached, then fully unhealthy
            HealthState::Unhealthy { .. } => HealthState::Unhealthy {
                consecutive_failures: health_params.failure_threshold,
                last_failure: Instant::now(),
            },
        };

        // If we just crossed into unhealthy, check if we need to force the circuit open...
        if let HealthState::Unhealthy {
            consecutive_failures,
            ..
        } = *entry
            && consecutive_failures >= health_params.failure_threshold
            && let Some(params) = self.circuit_params.get(service_id)
            && params.enable
        {
            let mut cb = self
                .circuit
                .entry((service_id.clone(), *upstream_id))
                .or_default();

            if cb.state() != CircuitState::Open {
                cb.trip_open((service_id, upstream_id), &params, "health_failed");
            }
        }
    }

    /// Any success will fully restore health
    pub(crate) fn report_success(
        &self,
        service_id: &ServiceId,
        upstream_id: &UpstreamId,
        latency: Duration,
    ) {
        let key = (service_id.clone(), *upstream_id);

        // Mark upstream healthy. Nothing reads the state while health checks are off, and an
        // upstream that is already healthy needs no write, so most successes take only a read.
        let health_enabled = self
            .health_params
            .get(service_id)
            .is_some_and(|params| params.enable);
        if health_enabled {
            let already_healthy = self
                .upstream_health
                .get(&key)
                .is_some_and(|state| matches!(*state, HealthState::Healthy));
            if !already_healthy {
                self.upstream_health
                    .insert(key.clone(), HealthState::Healthy);
            }
        }

        // Increment success counter
        let total = self
            .total_successes
            .entry(key.clone())
            .or_insert_with(|| AtomicU64::new(0));
        total.fetch_add(1, Ordering::Relaxed);

        // Update EWMA latency
        const ALPHA: f64 = 0.2;

        self.latency_stats
            .entry(key)
            .and_modify(|stats| {
                let old = stats.ewma.as_secs_f64();
                let new = latency.as_secs_f64();
                let updated = ALPHA * new + (1.0 - ALPHA) * old;
                stats.ewma = Duration::from_secs_f64(updated);
            })
            .or_insert_with(|| LatencyStats { ewma: latency });
    }

    /// Determines whether an upstream may receive a request
    pub fn health_status(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> HealthStatus {
        // A reload can remove the service while one of its requests is still in flight.
        let Some(health_params) = self.health_params.get(service_id) else {
            return HealthStatus { healthy: true };
        };

        if !health_params.enable {
            // Assume always healthy if health checks are disabled for this service.
            return HealthStatus { healthy: true };
        }

        let key = (service_id.clone(), *upstream_id);

        let healthy = if let Some(mut entry) = self.upstream_health.get_mut(&key) {
            match &*entry {
                HealthState::Healthy => true,

                HealthState::Unhealthy {
                    consecutive_failures,
                    ..
                } if *consecutive_failures < health_params.failure_threshold => true,

                HealthState::Unhealthy { last_failure, .. }
                    if last_failure.elapsed() > health_params.unhealthy_cooldown =>
                {
                    // Atomic promotion to Trial
                    *entry = HealthState::Unhealthy {
                        consecutive_failures: health_params.failure_threshold,
                        last_failure: Instant::now(),
                    };
                    true
                }

                _ => false,
            }
        } else {
            // Optimistic default
            true
        };

        HealthStatus { healthy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::traffic::{ServiceSnapshot, TrafficSnapshot};
    use snakeway_conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
    use std::collections::HashMap;

    fn manager_with_one_upstream(health_enabled: bool) -> (TrafficManager, ServiceId, UpstreamId) {
        manager_with_cooldown(health_enabled, 10)
    }

    fn manager_with_cooldown(
        health_enabled: bool,
        unhealthy_cooldown_seconds: u64,
    ) -> (TrafficManager, ServiceId, UpstreamId) {
        let service_id = ServiceId("api".into());
        let mut services = HashMap::new();
        services.insert(
            service_id.clone(),
            ServiceSnapshot {
                service_id: service_id.clone(),
                strategy: LoadBalancingStrategy::Failover,
                upstreams: vec![],
                circuit_breaker_cfg: CircuitBreakerConfig::default(),
                health_check_cfg: HealthCheckConfig {
                    enable: health_enabled,
                    failure_threshold: 3,
                    unhealthy_cooldown_seconds,
                },
                metric_service: crate::execution::traffic::snapshot::service_metric_attribute(
                    &std::sync::Arc::from("test"),
                ),
            },
        );
        let manager = TrafficManager::new(TrafficSnapshot { services });
        (manager, service_id, UpstreamId::new(1))
    }

    #[test]
    fn with_health_checks_off_a_success_records_no_health_state() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(false);

        // Act
        manager.report_success(&service, &upstream, Duration::from_millis(5));

        // Assert
        assert!(
            manager
                .upstream_health
                .get(&(service.clone(), upstream))
                .is_none()
        );
        assert_eq!(manager.total_successes(&service, &upstream), 1);
        assert!(manager.health_status(&service, &upstream).healthy);
    }

    #[test]
    fn with_health_checks_on_a_success_restores_an_unhealthy_upstream() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(true);
        for _ in 0..3 {
            manager.report_failure(&service, &upstream);
        }
        assert!(!manager.health_status(&service, &upstream).healthy);

        // Act
        manager.report_success(&service, &upstream, Duration::from_millis(5));

        // Assert
        assert!(manager.health_status(&service, &upstream).healthy);
        assert!(matches!(
            *manager
                .upstream_health
                .get(&(service.clone(), upstream))
                .unwrap(),
            HealthState::Healthy
        ));
        assert_eq!(manager.total_successes(&service, &upstream), 1);
    }

    #[test]
    fn with_health_checks_on_a_first_success_records_a_healthy_state() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(true);

        // Act
        manager.report_success(&service, &upstream, Duration::from_millis(5));

        // Assert
        assert!(matches!(
            *manager
                .upstream_health
                .get(&(service.clone(), upstream))
                .unwrap(),
            HealthState::Healthy
        ));
        assert!(manager.health_status(&service, &upstream).healthy);
    }

    #[test]
    fn a_failure_reported_after_a_reload_removes_the_service_records_nothing() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(true);
        manager.update(TrafficSnapshot::default());

        // Act
        manager.report_failure(&service, &upstream);

        // Assert
        assert_eq!(manager.total_failures(&service, &upstream), 0);
        assert!(manager.total_failures.is_empty());
        assert!(manager.upstream_health.is_empty());
        assert!(manager.circuit.is_empty());
    }

    #[test]
    fn a_service_that_a_reload_removes_reports_healthy() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(true);
        manager.update(TrafficSnapshot::default());

        // Act
        let status = manager.health_status(&service, &upstream);

        // Assert
        assert!(status.healthy);
        assert!(manager.upstream_health.is_empty());
    }

    #[test]
    fn with_the_circuit_breaker_off_an_upstream_that_health_checks_remove_returns_after_the_cooldown()
     {
        // Arrange
        let (manager, service, upstream) = manager_with_cooldown(true, 0);
        for _ in 0..3 {
            manager.report_failure(&service, &upstream);
        }
        std::thread::sleep(Duration::from_millis(5));

        // Act
        let healthy = manager.health_status(&service, &upstream).healthy;

        // Assert
        assert!(healthy);
        assert_ne!(
            manager.circuit_state(&service, &upstream),
            Some(CircuitState::Open)
        );
        assert!(manager.circuit_allows(&service, &upstream));
    }

    #[test]
    fn an_upstream_stays_healthy_until_its_failures_reach_the_threshold() {
        // Arrange
        let (manager, service, upstream) = manager_with_one_upstream(true);
        manager.report_failure(&service, &upstream);
        manager.report_failure(&service, &upstream);
        let below_threshold = manager.health_status(&service, &upstream).healthy;

        // Act
        manager.report_failure(&service, &upstream);

        // Assert
        assert!(below_threshold);
        assert!(!manager.health_status(&service, &upstream).healthy);
        assert!(matches!(
            *manager
                .upstream_health
                .get(&(service.clone(), upstream))
                .unwrap(),
            HealthState::Unhealthy {
                consecutive_failures: 3,
                ..
            }
        ));
    }
}
