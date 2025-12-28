use crate::ctx::RequestCtx;
use crate::traffic::{decision::*, snapshot::*, strategy::TrafficStrategy};

#[derive(Debug, Default)]
pub struct Failover {}

impl TrafficStrategy for Failover {
    fn decide(&self, _req: &RequestCtx, _healthy: &[UpstreamSnapshot]) -> Option<TrafficDecision> {
        todo!("implement failover")
    }
}
