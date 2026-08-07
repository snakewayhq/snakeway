use crate::execution::traffic::{ServiceId, TrafficManager, UpstreamSnapshot, WrrState};
use crate::runtime::UpstreamId;
use std::sync::atomic::{AtomicU64, Ordering};

/// Request Counters
impl TrafficManager {
    pub(crate) fn on_request_start(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);

        let counter = self
            .active_requests
            .entry(key.clone())
            .or_insert_with(|| AtomicU64::new(0));

        counter.fetch_add(1, Ordering::Relaxed);

        let total = self
            .total_requests
            .entry(key)
            .or_insert_with(|| AtomicU64::new(0));
        total.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn on_request_end(&self, service_id: &ServiceId, upstream_id: &UpstreamId) {
        let key = (service_id.clone(), *upstream_id);

        if let Some(counter) = self.active_requests.get(&key) {
            let prev = counter.fetch_sub(1, Ordering::Relaxed);
            if prev <= 1 {
                counter.store(0, Ordering::Relaxed);
            }
        }
    }

    pub fn active_requests(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u64 {
        self.active_requests
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub(crate) fn next_wrr_index(
        &self,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
    ) -> usize {
        debug_assert!(!healthy.is_empty());

        // One allocation per call. Hashing the ids in place would remove it if this
        // shows up in a profile.
        let upstream_ids: Vec<UpstreamId> = healthy.iter().map(|u| u.endpoint.id()).collect();

        let total_weight: i64 = healthy
            .iter()
            .map(|u| u.weight as i64) // assumes UpstreamSnapshot has `weight: u64`
            .sum();

        // Safety net: weight is enforced in config validation.
        debug_assert!(total_weight > 0);

        let mut entry = self
            .wrr_state
            .entry(service_id.clone())
            .or_insert_with(|| WrrState {
                current_weights: vec![0; healthy.len()],
                upstream_ids: upstream_ids.clone(),
                total_weight,
            });

        // Reset if upstream set/order or weights changed
        if entry.current_weights.len() != healthy.len()
            || entry.total_weight != total_weight
            || entry.upstream_ids != upstream_ids
        {
            entry.current_weights = vec![0; healthy.len()];
            entry.upstream_ids = upstream_ids;
            entry.total_weight = total_weight;
        }

        // Smooth Weighted Round Robin
        let mut best_idx = 0usize;
        let mut best_val = i64::MIN;

        for (i, u) in healthy.iter().enumerate() {
            let w = u.weight as i64;
            entry.current_weights[i] += w;

            if entry.current_weights[i] > best_val {
                best_val = entry.current_weights[i];
                best_idx = i;
            }
        }

        entry.current_weights[best_idx] -= entry.total_weight;

        best_idx
    }
}
