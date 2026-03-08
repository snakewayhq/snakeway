use crate::ctx::RequestCtx;
use crate::traffic_management::{
    ServiceId, TrafficManager, decision::*, snapshot::*, strategy::TrafficStrategy,
};

#[derive(Debug, Default)]
pub struct Failover {}

impl TrafficStrategy for Failover {
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

        let healthy = healthy.first()?;

        Some(TrafficDecision {
            upstream_id: healthy.endpoint.id(),
            reason: DecisionReason::Failover,
            cb_started: true,
        })
    }
}
