use crate::conf::types::Strategy;
use crate::server::UpstreamRuntime;
use crate::traffic::types::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UpstreamSnapshot {
    pub endpoint: UpstreamRuntime,
    pub latency: Option<LatencyStats>,
    pub connections: ConnectionStats,
    pub health: HealthStatus,
}

#[derive(Debug, Clone)]
pub struct ServiceSnapshot {
    pub service_id: ServiceId,
    pub strategy: Strategy,
    pub upstreams: Vec<UpstreamSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct TrafficSnapshot {
    pub services: HashMap<ServiceId, ServiceSnapshot>,
}
