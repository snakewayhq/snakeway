use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};

#[derive(Debug, Default)]
pub struct Failover {}

impl TrafficStrategy for Failover {
    fn decide(&self, _req: &RequestCtx, healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let healthy = healthy.first()?;

        Some(TrafficDecision {
            upstream_id: healthy.endpoint.id,
            reason: DecisionReason::Failover,
            protocol: None,
        })
    }
}
