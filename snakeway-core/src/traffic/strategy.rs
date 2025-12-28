use crate::ctx::RequestCtx;
use crate::traffic::decision::TrafficDecision;
use crate::traffic::{ServiceId, TrafficManager, UpstreamSnapshot};

pub trait TrafficStrategy: Send + Sync {
    fn decide(
        &self,
        req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision>;
}
