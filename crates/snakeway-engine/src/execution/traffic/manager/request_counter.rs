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
        healthy: &[&UpstreamSnapshot],
    ) -> usize {
        debug_assert!(!healthy.is_empty());

        let total_weight: i64 = healthy
            .iter()
            .map(|u| u.weight as i64) // assumes UpstreamSnapshot has `weight: u64`
            .sum();

        // Safety net: weight is enforced in config validation.
        debug_assert!(total_weight > 0);

        let mut entry = match self.wrr_state.get_mut(service_id) {
            Some(entry) => entry,
            None => self
                .wrr_state
                .entry(service_id.clone())
                .or_insert_with(|| WrrState {
                    current_weights: Vec::new(),
                    upstream_ids: Vec::new(),
                    total_weight: 0,
                }),
        };

        // Reset if upstream set/order or weights changed. The buffers are refilled in place.
        let stale = entry.current_weights.len() != healthy.len()
            || entry.total_weight != total_weight
            || !entry
                .upstream_ids
                .iter()
                .copied()
                .eq(healthy.iter().map(|u| u.endpoint.id()));

        if stale {
            entry.current_weights.clear();
            entry.current_weights.resize(healthy.len(), 0);
            entry.upstream_ids.clear();
            entry
                .upstream_ids
                .extend(healthy.iter().map(|u| u.endpoint.id()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::traffic::TrafficSnapshot;
    use crate::runtime::{ResolvedAddr, UpstreamRuntime, UpstreamTcpRuntime};

    fn upstream_snapshot_with_weight(id: u16, weight: u32) -> UpstreamSnapshot {
        UpstreamSnapshot {
            endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                id: UpstreamId::new(id.into()),
                host: "127.0.0.1".to_string(),
                port: id,
                resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], id).into()),
                tls: None,
                weight,
            }),
            weight,
            metric_attributes: crate::execution::traffic::snapshot::upstream_metric_attributes(
                &std::sync::Arc::from("test"),
                crate::runtime::UpstreamId::new(id.into()),
            ),
        }
    }

    fn collect_wrr_indices(
        manager: &TrafficManager,
        service: &ServiceId,
        healthy: &[&UpstreamSnapshot],
        n: usize,
    ) -> Vec<usize> {
        (0..n)
            .map(|_| manager.next_wrr_index(service, healthy))
            .collect()
    }

    #[test]
    fn weighted_round_robin_follows_the_smooth_order_over_one_cycle() {
        // Arrange
        let manager = TrafficManager::new(TrafficSnapshot::default());
        let service = ServiceId("api".into());
        let owned = [upstream_snapshot_with_weight(1, 5), upstream_snapshot_with_weight(2, 1), upstream_snapshot_with_weight(3, 1)];
        let healthy: Vec<&UpstreamSnapshot> = owned.iter().collect();

        // Act
        let order = collect_wrr_indices(&manager, &service, &healthy, 7);

        // Assert
        assert_eq!(order, vec![0, 0, 1, 0, 2, 0, 0]);
    }

    #[test]
    fn repeating_the_cycle_with_the_same_set_gives_the_same_order() {
        // Arrange
        let manager = TrafficManager::new(TrafficSnapshot::default());
        let service = ServiceId("api".into());
        let owned = [upstream_snapshot_with_weight(1, 5), upstream_snapshot_with_weight(2, 1), upstream_snapshot_with_weight(3, 1)];
        let healthy: Vec<&UpstreamSnapshot> = owned.iter().collect();

        // Act
        let order = collect_wrr_indices(&manager, &service, &healthy, 14);

        // Assert
        assert_eq!(order, vec![0, 0, 1, 0, 2, 0, 0, 0, 0, 1, 0, 2, 0, 0]);
    }

    #[test]
    fn a_changed_upstream_set_restarts_from_a_fresh_state() {
        // Arrange
        let manager = TrafficManager::new(TrafficSnapshot::default());
        let fresh = TrafficManager::new(TrafficSnapshot::default());
        let service = ServiceId("api".into());
        let owned_before = [upstream_snapshot_with_weight(1, 5), upstream_snapshot_with_weight(2, 1), upstream_snapshot_with_weight(3, 1)];
        let owned_after = [upstream_snapshot_with_weight(1, 5), upstream_snapshot_with_weight(4, 1)];
        let before: Vec<&UpstreamSnapshot> = owned_before.iter().collect();
        let after: Vec<&UpstreamSnapshot> = owned_after.iter().collect();
        collect_wrr_indices(&manager, &service, &before, 3);

        // Act
        let order = collect_wrr_indices(&manager, &service, &after, 6);

        // Assert
        assert_eq!(order, collect_wrr_indices(&fresh, &service, &after, 6));
        assert_eq!(order, vec![0, 0, 0, 1, 0, 0]);
    }

    #[test]
    fn a_steady_state_pick_does_not_allocate() {
        // Arrange
        let manager = TrafficManager::new(TrafficSnapshot::default());
        let service = ServiceId("api".into());
        let owned = [upstream_snapshot_with_weight(1, 5), upstream_snapshot_with_weight(2, 1), upstream_snapshot_with_weight(3, 1)];
        let healthy: Vec<&UpstreamSnapshot> = owned.iter().collect();
        manager.next_wrr_index(&service, &healthy);

        // Act
        let (index, allocations) =
            crate::alloc_count::count_allocations(|| manager.next_wrr_index(&service, &healthy));

        // Assert
        assert_eq!(allocations, 0);
        assert_eq!(index, 0);
    }
}
