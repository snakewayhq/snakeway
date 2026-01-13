use crate::ctx::RequestCtx;
use crate::traffic_management::{
    ServiceId, TrafficManager, decision::*, snapshot::*, strategy::TrafficStrategy,
};

#[derive(Debug, Default)]
pub struct RequestPressure;

impl TrafficStrategy for RequestPressure {
    fn decide(
        &self,
        _req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision> {
        let upstream = healthy.iter().min_by_key(|u| {
            (
                traffic_manager.active_requests(service_id, &u.endpoint.id()),
                u.endpoint.id(), // Deterministic tie-break.
            )
        })?;

        Some(TrafficDecision {
            upstream_id: upstream.endpoint.id(),
            reason: DecisionReason::AdmissionPressure,
            cb_started: true,
        })
    }
}
