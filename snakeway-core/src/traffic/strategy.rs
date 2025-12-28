use crate::ctx::RequestCtx;
use crate::traffic::UpstreamSnapshot;
use crate::traffic::decision::TrafficDecision;

pub trait TrafficStrategy: Send + Sync {
    fn decide(&self, req: &RequestCtx, healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision>;
}
