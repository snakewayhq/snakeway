use crate::ctx::RequestCtx;
use crate::traffic_management::{
    ServiceId, TrafficManager, decision::*, snapshot::*, strategy::TrafficStrategy,
};
use rand::{Rng, rng};

#[derive(Debug, Default)]
pub struct Random {}

impl TrafficStrategy for Random {
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

        // This is per thread, which is ok for a random algorithm.
        let idx = rng().random_range(0..healthy.len());
        let upstream_snapshot = &healthy[idx];

        Some(TrafficDecision {
            upstream_id: upstream_snapshot.endpoint.id(),
            reason: DecisionReason::Random,
            protocol: None,
            cb_started: true,
        })
    }
}
