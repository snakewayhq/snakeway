use crate::runtime::{UpstreamId, UpstreamRuntime};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DecisionReason {
    Failover,
    RoundRobin,
    AdmissionPressure,
    Random,
    StickyHash,
    NoStrategyDecision,
}

#[derive(Debug, Clone)]
pub(crate) struct TrafficDecision {
    pub(crate) upstream_id: UpstreamId,
    pub(crate) reason: DecisionReason,
    pub(crate) cb_started: bool,
}

pub(crate) struct SelectedUpstream<'a> {
    pub(crate) upstream: &'a UpstreamRuntime,
    pub(crate) cb_started: bool,
}
