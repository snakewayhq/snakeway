use super::types::{HealthState, WrrState};
use crate::execution::traffic::circuit::{CircuitBreaker, CircuitBreakerParams};
use crate::execution::traffic::snapshot::TrafficSnapshot;
use crate::execution::traffic::{HealthCheckParams, LatencyStats, ServiceId};
use crate::runtime::UpstreamId;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

#[derive(Debug)]
pub struct TrafficManager {
    pub(crate) snapshot: ArcSwap<TrafficSnapshot>,

    /// Live per-upstream counters (hot path)
    pub(crate) active_requests: DashMap<(ServiceId, UpstreamId), AtomicU64>,

    /// Per-upstream weighted round-robin state
    pub(crate) wrr_state: DashMap<ServiceId, WrrState>,

    /// Per-upstream health state
    pub(crate) upstream_health: DashMap<(ServiceId, UpstreamId), HealthState>,

    /// Per-upstream counters
    pub(crate) total_requests: DashMap<(ServiceId, UpstreamId), AtomicU64>,
    pub(crate) total_successes: DashMap<(ServiceId, UpstreamId), AtomicU64>,
    pub(crate) total_failures: DashMap<(ServiceId, UpstreamId), AtomicU64>,
    pub(crate) latency_stats: DashMap<(ServiceId, UpstreamId), LatencyStats>,

    /// Per-upstream circuit breaker state machine
    pub(crate) circuit: DashMap<(ServiceId, UpstreamId), CircuitBreaker>,

    /// Per-service circuit breaker parameters (cloned from snapshot)
    pub(crate) circuit_params: DashMap<ServiceId, Arc<CircuitBreakerParams>>,

    /// Per-service health check parameters (cloned from snapshot)
    pub(crate) health_params: DashMap<ServiceId, Arc<HealthCheckParams>>,
}

impl TrafficManager {
    pub fn new(initial: TrafficSnapshot) -> Self {
        let tm = Self {
            snapshot: ArcSwap::from_pointee(initial.clone()),
            active_requests: DashMap::new(),
            wrr_state: DashMap::new(),
            upstream_health: DashMap::new(),
            total_requests: DashMap::new(),
            total_successes: DashMap::new(),
            total_failures: DashMap::new(),
            latency_stats: DashMap::new(),
            circuit: DashMap::new(),
            circuit_params: DashMap::new(),
            health_params: DashMap::new(),
        };

        tm.update(initial);

        tm
    }
}
