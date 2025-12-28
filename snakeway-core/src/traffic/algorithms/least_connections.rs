use crate::ctx::RequestCtx;
use crate::traffic::{
    ServiceId, TrafficManager, decision::*, snapshot::*, strategy::TrafficStrategy,
};

#[derive(Debug, Default)]
pub struct LeastConnections;

impl TrafficStrategy for LeastConnections {
    fn decide(
        &self,
        _req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision> {
        let upstream = healthy
            .iter()
            .min_by_key(|u| traffic_manager.active_requests(&service_id, &u.endpoint.id))?;

        Some(TrafficDecision {
            upstream_id: upstream.endpoint.id,
            reason: DecisionReason::LeastConnections,
            protocol: None,
        })
    }
}
