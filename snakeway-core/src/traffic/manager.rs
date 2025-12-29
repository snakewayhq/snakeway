use crate::server::UpstreamId;
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

#[derive(Debug)]
struct HealthState {
    healthy: bool,
    consecutive_failures: u32,
    last_failure: Instant,
}

#[derive(Debug)]
pub struct TrafficManager {
    snapshot: ArcSwap<TrafficSnapshot>,

    /// Live per-upstream counters (hot path)
    active_requests: DashMap<(ServiceId, UpstreamId), AtomicU32>,

    /// Per-service round-robin cursors
    rr_cursors: DashMap<ServiceId, AtomicUsize>,

    /// Per-upstream health state.
    upstream_health: DashMap<(ServiceId, UpstreamId), HealthState>,
}

impl Default for TrafficManager {
    fn default() -> Self {
        Self::new(TrafficSnapshot::default())
    }
}

impl TrafficManager {
    pub fn new(initial: TrafficSnapshot) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(initial),
            active_requests: DashMap::new(),
            rr_cursors: DashMap::new(),
            upstream_health: Default::default(),
        }
    }
}

/// Snapshot API
impl TrafficManager {
    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
        // Extract valid service IDs.
        let valid_services: HashSet<ServiceId> = new_snapshot.services.keys().cloned().collect();

        // Cleanup round-robin cursors.
        self.rr_cursors
            .retain(|service_id, _| valid_services.contains(service_id));

        // Clean active request counters.
        self.active_requests.retain(|(service_id, upstream_id), _| {
            if let Some(service) = new_snapshot.services.get(service_id) {
                service
                    .upstreams
                    .iter()
                    .any(|u| u.endpoint.id == *upstream_id)
            } else {
                false
            }
        });

        // Cleanup upstream health state.
        self.upstream_health.retain(|(service_id, upstream_id), _| {
            new_snapshot
                .services
                .get(service_id)
                .map(|svc| svc.upstreams.iter().any(|u| u.endpoint.id == *upstream_id))
                .unwrap_or(false)
        });

        // Atomically publish snapshot
        self.snapshot.store(Arc::new(new_snapshot));
    }

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

            // Defensive: avoid underflow in case of bugs
            if prev <= 1 {
                counter.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Helper to get the active requests count for a specific upstream.
    /// Returns 0 if the upstream is not found.
    pub fn active_requests(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u32 {
        let key = (service_id.clone(), *upstream_id);

        self.active_requests
            .get(&key)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Gets the next round-robin index for the specified service.
    ///
    /// This method maintains a per-service cursor that increments atomically on each call,
    /// ensuring fair distribution of requests across upstreams in a round-robin fashion.
    ///
    /// # Arguments
    /// * `service_id` - The service identifier to get the cursor for
    /// * `modulo` - The number of upstreams to distribute across (returns index % modulo)
    ///
    /// # Returns
    /// The next index in the round-robin sequence, wrapped by the modulo value
    pub fn next_rr_index(&self, service_id: &ServiceId, modulo: usize) -> usize {
        let cursor = self
            .rr_cursors
            .entry(service_id.clone())
            .or_insert_with(|| AtomicUsize::new(0));

        cursor.fetch_add(1, Ordering::Relaxed) % modulo
    }
}

/// Health status API
impl TrafficManager {
    pub fn report_failure(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);

        let mut entry = self
            .upstream_health
            .entry(key)
            .or_insert_with(|| HealthState {
                healthy: true,
                consecutive_failures: 0,
                last_failure: Instant::now(),
            });

        entry.consecutive_failures += 1;
        entry.last_failure = Instant::now();

        if entry.consecutive_failures >= FAILURE_THRESHOLD {
            entry.healthy = false;
        }
    }

    pub fn report_success(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        if let Some(mut entry) = self
            .upstream_health
            .get_mut(&(service_id.clone(), *upstream_id))
        {
            entry.consecutive_failures = 0;
            entry.healthy = true;
        }
    }

    pub fn health_status(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> HealthStatus {
        let healthy = self
            .upstream_health
            .get(&(service_id.clone(), *upstream_id))
            .map(|h| {
                if !h.healthy && h.last_failure.elapsed() > UNHEALTHY_COOLDOWN {
                    // optimistic retry
                    true
                } else {
                    h.healthy
                }
            })
            .unwrap_or(true);

        HealthStatus { healthy }
    }
}
