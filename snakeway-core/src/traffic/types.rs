use std::time::Duration;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ServiceId(pub String);

#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Exponential weighted moving average of latency
    pub ewma: Duration,
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub active: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthStatus {
    pub healthy: bool,
}
