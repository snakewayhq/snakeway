use crate::ctx::RequestCtx;
use crate::traffic::{
    ServiceId, TrafficManager,
    decision::{DecisionReason, TrafficDecision},
    snapshot::UpstreamSnapshot,
    strategy::TrafficStrategy,
};

#[derive(Debug, Default)]
pub struct RoundRobin;

impl TrafficStrategy for RoundRobin {
    fn decide(
        &self,
        _req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let idx = traffic_manager.next_rr_index(service_id, healthy.len());

        let upstream_snapshot = &healthy[idx];

        Some(TrafficDecision {
            upstream_id: upstream_snapshot.endpoint.id,
            reason: DecisionReason::RoundRobin,
            protocol: None,
            cb_started: true,
        })
    }
}
