use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};

#[derive(Debug, Default)]
pub struct LeastConnections;

impl TrafficStrategy for LeastConnections {
    fn decide(&self, _req: &RequestCtx, healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        let upstream = healthy.iter().min_by_key(|u| u.connections.active)?;

        Some(TrafficDecision {
            upstream_id: upstream.endpoint.id,
            reason: DecisionReason::LeastConnections,
            protocol: None,
        })
    }
}
