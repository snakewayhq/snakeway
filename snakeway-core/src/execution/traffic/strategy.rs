use crate::ctx::RequestCtx;
use crate::traffic_management::decision::TrafficDecision;
use crate::traffic_management::{ServiceId, TrafficManager, UpstreamSnapshot};

pub trait TrafficStrategy: Send + Sync {
    fn decide(
        &self,
        req: &RequestCtx,
        service_id: &ServiceId,
        healthy: &[UpstreamSnapshot],
        traffic_manager: &TrafficManager,
    ) -> Option<TrafficDecision>;
}
