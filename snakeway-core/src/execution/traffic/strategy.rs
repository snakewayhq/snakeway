use crate::execution::ctx::RequestCtx;
use crate::execution::traffic::decision::TrafficDecision;
use crate::execution::traffic::{ServiceId, TrafficManager, UpstreamSnapshot};

pub(crate) trait TrafficStrategy: Send + Sync {
    fn decide(
        &self,
        req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision>;
}
