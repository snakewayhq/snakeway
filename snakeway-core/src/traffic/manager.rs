use crate::server::UpstreamId;
use crate::traffic::circuit::{CircuitBreaker, CircuitBreakerParams, CircuitState};
use crate::traffic::snapshot::TrafficSnapshot;
use crate::traffic::{HealthStatus, ServiceId};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const FAILURE_THRESHOLD: u32 = 3;
const UNHEALTHY_COOLDOWN: Duration = Duration::from_secs(10);

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

    /// Per-upstream circuit breaker state machine
    pub circuit: DashMap<(ServiceId, UpstreamId), CircuitBreaker>,

    /// Per-service circuit breaker parameters (cloned from snapshot)
    pub circuit_params: DashMap<ServiceId, Arc<CircuitBreakerParams>>,
}

impl TrafficManager {
    pub fn new(initial: TrafficSnapshot) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(initial),
            active_requests: DashMap::new(),
            rr_cursors: DashMap::new(),
            upstream_health: DashMap::new(),
            circuit: DashMap::new(),
            circuit_params: DashMap::new(),
        }
    }
}

/// Snapshot API (read-only)
impl TrafficManager {
    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
        let valid_services: HashSet<ServiceId> = new_snapshot.services.keys().cloned().collect();

        // Cleanup round-robin cursors
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

        // Cleanup circuit breaker state
        self.circuit.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });

        // Cleanup and update circuit breaker parameters
        self.circuit_params
            .retain(|service_id, _| valid_services.contains(service_id));

        for (svc_id, svc) in new_snapshot.services.iter() {
            self.circuit_params
                .insert(svc_id.clone(), Arc::new(svc.circuit.clone()));
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
            .entry(key)
            .or_insert_with(|| AtomicU32::new(0));

        counter.fetch_add(1, Ordering::Relaxed);
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
        let key = (service_id.clone(), *upstream_id);

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
            } if consecutive_failures + 1 < FAILURE_THRESHOLD => HealthState::Unhealthy {
                consecutive_failures: consecutive_failures + 1,
                last_failure: Instant::now(),
            },

            // Threshold reached, then fully unhealthy
            HealthState::Unhealthy { .. } => HealthState::Unhealthy {
                consecutive_failures: FAILURE_THRESHOLD,
                last_failure: Instant::now(),
            },
        };
    }

    /// Any success will fully restore health
    pub fn report_success(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        self.upstream_health
            .insert((service_id.clone(), *upstream_id), HealthState::Healthy);
    }

    /// Determines whether an upstream may receive a request
    pub fn health_status(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> HealthStatus {
        let key = (service_id.clone(), *upstream_id);

        let healthy = if let Some(mut entry) = self.upstream_health.get_mut(&key) {
            match &*entry {
                HealthState::Healthy => true,

                HealthState::Unhealthy { last_failure, .. }
                    if last_failure.elapsed() > UNHEALTHY_COOLDOWN =>
                {
                    // Atomic promotion to Trial
                    *entry = HealthState::Unhealthy {
                        consecutive_failures: FAILURE_THRESHOLD,
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
        entry.allow_request(&params)
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
        entry.on_request_end(&params, started, success);
    }

    pub fn circuit_state(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> CircuitState {
        self.circuit
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.state())
            .unwrap_or(CircuitState::Closed)
    }
}
