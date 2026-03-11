use crate::runtime::{UpstreamId, UpstreamRuntime};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DecisionReason {
    Failover,
    RoundRobin,
    AdmissionPressure,
    Random,
    StickyHash,
    NoStrategyDecision,
}

impl Display for DecisionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionReason::Failover => write!(f, "Failover"),
            DecisionReason::RoundRobin => write!(f, "RoundRobin"),
            DecisionReason::AdmissionPressure => write!(f, "AdmissionPressure"),
            DecisionReason::Random => write!(f, "Random"),
            DecisionReason::StickyHash => write!(f, "StickyHash"),
            DecisionReason::NoStrategyDecision => write!(f, "NoStrategyDecision"),
        }
    }
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
