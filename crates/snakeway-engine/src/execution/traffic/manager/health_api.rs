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
        let health_params = self.health_params.get(service_id).unwrap_or_else(|| {
            unreachable!(
                "health params missing for service {} — invariant violated",
                service_id
            )
        });

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
        {
            let mut cb = self
                .circuit
                .entry((service_id.clone(), *upstream_id))
                .or_default();

            if cb.state() != CircuitState::Open {
                // Health failures are allowed to force the circuit open,
                // even when auto-recovery is disabled. In that case, only
                // health recovery can close it again.
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

        // Mark upstream healthy
        self.upstream_health
            .insert(key.clone(), HealthState::Healthy);

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
        let health_params = self.health_params.get(service_id).unwrap_or_else(|| {
            unreachable!(
                "health params missing for service {} — invariant violated",
                service_id
            )
        });

        if !health_params.enable {
            // Assume always healthy if health checks are disabled for this service.
            return HealthStatus { healthy: true };
        }

        let key = (service_id.clone(), *upstream_id);

        let healthy = if let Some(mut entry) = self.upstream_health.get_mut(&key) {
            match &*entry {
                HealthState::Healthy => true,

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
