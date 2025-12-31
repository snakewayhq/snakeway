use crate::server::UpstreamId;
use crate::traffic::admin::{
    AdminUpstreamView, CircuitBreakerDetailsView, CircuitBreakerParamsView,
};
use crate::traffic::circuit::{CircuitBreaker, CircuitBreakerParams, CircuitState};
use crate::traffic::snapshot::TrafficSnapshot;
use crate::traffic::{HealthCheckParams, HealthStatus, ServiceId};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum UpstreamOutcome {
    TransportError,
    HttpStatus(u16),
    Success,
}

/// Health state of an upstream endpoint
#[derive(Debug, Clone)]
enum HealthState {
    Healthy,
    Unhealthy {
        consecutive_failures: u32,
        last_failure: Instant,
    },
}

#[derive(Debug)]
pub struct TrafficManager {
    snapshot: ArcSwap<TrafficSnapshot>,

    /// Live per-upstream counters (hot path)
    active_requests: DashMap<(ServiceId, UpstreamId), AtomicU32>,

    /// Per-service round-robin cursors
    rr_cursors: DashMap<ServiceId, AtomicUsize>,

    /// Per-upstream health state
    upstream_health: DashMap<(ServiceId, UpstreamId), HealthState>,

    /// Per-upstream counters
    total_requests: DashMap<(ServiceId, UpstreamId), AtomicU32>,
    total_successes: DashMap<(ServiceId, UpstreamId), AtomicU32>,
    total_failures: DashMap<(ServiceId, UpstreamId), AtomicU32>,

    /// Per-upstream circuit breaker state machine
    pub circuit: DashMap<(ServiceId, UpstreamId), CircuitBreaker>,

    /// Per-service circuit breaker parameters (cloned from snapshot)
    pub circuit_params: DashMap<ServiceId, Arc<CircuitBreakerParams>>,

    /// Per-service health check parameters (cloned from snapshot)
    pub health_params: DashMap<ServiceId, Arc<HealthCheckParams>>,
}

impl TrafficManager {
    pub fn new(initial: TrafficSnapshot) -> Self {
        let tm = Self {
            snapshot: ArcSwap::from_pointee(initial.clone()),
            active_requests: DashMap::new(),
            rr_cursors: DashMap::new(),
            upstream_health: DashMap::new(),
            total_requests: DashMap::new(),
            total_successes: DashMap::new(),
            total_failures: DashMap::new(),
            circuit: DashMap::new(),
            circuit_params: DashMap::new(),
            health_params: DashMap::new(),
        };

        tm.update(initial);

        tm
    }
}

/// Snapshot API (read-only)
impl TrafficManager {
    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
        let valid_services: HashSet<ServiceId> = new_snapshot.services.keys().cloned().collect();

        // Clean up round-robin cursors
        self.rr_cursors
            .retain(|service_id, _| valid_services.contains(service_id));

        // Cleanup active request counters
        self.active_requests.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });

        // Cleanup health state
        self.upstream_health.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });

        // Cleanup total counters
        self.total_requests.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });
        self.total_successes.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });
        self.total_failures.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });

        // Cleanup circuit breaker state
        self.circuit.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
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
                open_duration: Duration::from_millis(svc.circuit_breaker_cfg.open_duration_ms),
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

/// Request Counters
impl TrafficManager {
    pub fn on_request_start(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);

        let counter = self
            .active_requests
            .entry(key.clone())
            .or_insert_with(|| AtomicU32::new(0));

        counter.fetch_add(1, Ordering::Relaxed);

        let total = self
            .total_requests
            .entry(key)
            .or_insert_with(|| AtomicU32::new(0));
        total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn on_request_end(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);

        if let Some(counter) = self.active_requests.get(&key) {
            let prev = counter.fetch_sub(1, Ordering::Relaxed);
            if prev <= 1 {
                counter.store(0, Ordering::Relaxed);
            }
        }
    }

    pub fn active_requests(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u32 {
        self.active_requests
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn next_rr_index(&self, service_id: &ServiceId, modulo: usize) -> usize {
        let cursor = self
            .rr_cursors
            .entry(service_id.clone())
            .or_insert_with(|| AtomicUsize::new(0));

        cursor.fetch_add(1, Ordering::Relaxed) % modulo
    }
}

/// Health API
impl TrafficManager {
    pub fn report_failure(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let health_params = self.health_params.get(service_id).unwrap_or_else(|| {
            unreachable!(
                "health params missing for service {} — invariant violated",
                service_id
            )
        });

        if !health_params.enable {
            // Health checks are disabled for this service, so we can't report failures.
            return;
        }

        let key = (service_id.clone(), *upstream_id);

        let total = self
            .total_failures
            .entry(key.clone())
            .or_insert_with(|| AtomicU32::new(0));
        total.fetch_add(1, Ordering::Relaxed);

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
    pub fn report_success(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);
        self.upstream_health
            .insert(key.clone(), HealthState::Healthy);

        let total = self
            .total_successes
            .entry(key)
            .or_insert_with(|| AtomicU32::new(0));
        total.fetch_add(1, Ordering::Relaxed);
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

/// Circuit Breaker API
impl TrafficManager {
    /// Called by director when selecting an upstream.
    pub fn circuit_allows(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> bool {
        let params = match self.circuit_params.get(service_id) {
            Some(p) => p.clone(),
            None => return true, // fail-open: no config means no circuit
        };

        let key = (service_id.clone(), *upstream_id);
        let mut entry = self.circuit.entry(key).or_default();
        entry.allow_request((service_id, upstream_id), &params)
    }

    /// Called once per request, after we know whether it succeeded.
    /// `started` must be true only if `circuit_allows()` returned true for this request.
    pub fn circuit_on_end(
        &self,
        service_id: &ServiceId,
        upstream_id: &UpstreamId,
        started: bool,
        success: bool,
    ) {
        let params = match self.circuit_params.get(service_id) {
            Some(p) => p.clone(),
            None => return,
        };

        let key = (service_id.clone(), *upstream_id);
        let mut entry = self.circuit.entry(key).or_default();
        entry.on_request_end((service_id, upstream_id), &params, started, success);
    }

    pub fn circuit_state(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> CircuitState {
        self.circuit
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.state())
            .unwrap_or(CircuitState::Closed)
    }

    pub fn total_requests(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u32 {
        self.total_requests
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn total_successes(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u32 {
        self.total_successes
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn total_failures(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u32 {
        self.total_failures
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn get_upstream_view(
        &self,
        service_id: &ServiceId,
        upstream_id: &UpstreamId,
        include_details: bool,
    ) -> AdminUpstreamView {
        let health = self.health_status(service_id, upstream_id);
        let active_requests = self.active_requests(service_id, upstream_id);

        let (total_requests, total_successes, total_failures) = if include_details {
            (
                self.total_requests(service_id, upstream_id),
                self.total_successes(service_id, upstream_id),
                self.total_failures(service_id, upstream_id),
            )
        } else {
            (0, 0, 0)
        };

        let circuit_params = if include_details {
            self.circuit_params
                .get(service_id)
                .map(|p| CircuitBreakerParamsView::from(&**p))
        } else {
            None
        };

        let (circuit_state, circuit_details) = self
            .circuit
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| {
                let details = if include_details {
                    Some(CircuitBreakerDetailsView {
                        consecutive_failures: c.consecutive_failures,
                        opened_at_rfc3339: c
                            .opened_at_system
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                        half_open_in_flight: c.half_open_in_flight,
                        half_open_successes: c.half_open_successes,
                    })
                } else {
                    None
                };
                (c.state(), details)
            })
            .unwrap_or((CircuitState::Closed, None));

        AdminUpstreamView {
            health,
            circuit: circuit_state,
            active_requests,
            total_requests,
            total_successes,
            total_failures,
            circuit_params,
            circuit_details,
        }
    }
}
