use crate::server::{UpstreamId, UpstreamRuntime};

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
pub enum EnforcedProtocol {
    Http1,
    Http2,
}

#[derive(Debug, Clone)]
pub struct TrafficDecision {
    pub upstream_id: UpstreamId,
    pub reason: DecisionReason,
    pub protocol: Option<EnforcedProtocol>,
    pub cb_started: bool,
}

pub struct SelectedUpstream<'a> {
    pub upstream: &'a UpstreamRuntime,
    pub cb_started: bool,
}
