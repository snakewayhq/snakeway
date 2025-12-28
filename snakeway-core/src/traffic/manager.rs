use crate::server::UpstreamId;
use crate::traffic::ServiceId;
use crate::traffic::snapshot::TrafficSnapshot;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct TrafficManager {
    snapshot: ArcSwap<TrafficSnapshot>,

    /// Live per-upstream counters (hot path)
    active_requests: DashMap<(ServiceId, UpstreamId), AtomicU32>,
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
        }
    }

    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn rebuild_snapshot(&self, base: &TrafficSnapshot) -> TrafficSnapshot {
        let mut new = base.clone();

        for entry in self.active_requests.iter() {
            let (service_id, upstream_id) = entry.key();
            let counter = entry.value();

            if let Some(service) = new.services.get_mut(service_id) {
                if let Some(upstream) = service
                    .upstreams
                    .iter_mut()
                    .find(|u| u.endpoint.id == *upstream_id)
                {
                    upstream.connections.active = counter.load(Ordering::Relaxed);
                }
            }
        }

        new
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
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
}
