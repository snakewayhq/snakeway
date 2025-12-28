use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};
use rand::{Rng, thread_rng};

#[derive(Debug, Default)]
pub struct Random {}

impl TrafficStrategy for Random {
    fn decide(&self, _req: &RequestCtx, healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        if healthy.is_empty() {
            return None;
        }

        let idx = thread_rng().gen_range(0..healthy.len());
        let upstream = &healthy[idx];

        Some(TrafficDecision {
            upstream_id: upstream.endpoint.id,
            reason: DecisionReason::Random,
            protocol: None,
        })
    }
}
