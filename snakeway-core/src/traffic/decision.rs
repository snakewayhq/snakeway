use crate::server::UpstreamId;

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionReason {
    Failover,
    RoundRobin,
    LeastConnections,
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
}
