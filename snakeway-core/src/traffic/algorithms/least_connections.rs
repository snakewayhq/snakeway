use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};

#[derive(Debug, Default)]
pub struct LeastConnections;

impl TrafficStrategy for LeastConnections {
    fn decide(&self, _req: &RequestCtx, _healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        todo!("implement least connections")
    }
}
