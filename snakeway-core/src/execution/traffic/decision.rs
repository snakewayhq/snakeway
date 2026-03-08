use crate::runtime::{UpstreamId, UpstreamRuntime};

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionReason {
    Failover,
    RoundRobin,
    AdmissionPressure,
    Random,
    StickyHash,
    NoStrategyDecision,
}

#[derive(Debug, Clone)]
pub struct TrafficDecision {
    pub upstream_id: UpstreamId,
    pub reason: DecisionReason,
    pub cb_started: bool,
}

pub struct SelectedUpstream<'a> {
    pub upstream: &'a UpstreamRuntime,
    pub cb_started: bool,
}
