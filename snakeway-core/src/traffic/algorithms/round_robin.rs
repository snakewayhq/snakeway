use crate::ctx::RequestCtx;
use crate::traffic::{
    ServiceId, TrafficManager,
    decision::{DecisionReason, TrafficDecision},
    snapshot::UpstreamSnapshot,
    strategy::TrafficStrategy,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct RoundRobin {
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl TrafficStrategy for RoundRobin {
    fn decide(
        &self,
        _req: &RequestCtx,
        _service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        _traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % healthy.len();
        let upstream_snapshot = &healthy[idx];

        Some(TrafficDecision {
            upstream_id: upstream_snapshot.endpoint.id,
            reason: DecisionReason::RoundRobin,
            protocol: None,
        })
    }
}
