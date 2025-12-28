use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpstreamId(pub u32);

#[derive(Debug, Clone)]
pub struct UpstreamEndpoint {
    pub id: UpstreamId,
    pub address: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ServiceId(pub String);

#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub exponentially_weighted_moving_average: Duration,
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub active: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthStatus {
    pub healthy: bool,
}
