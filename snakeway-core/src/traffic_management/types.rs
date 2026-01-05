use std::fmt::{Display, Formatter};
use std::time::Duration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize)]
pub struct ServiceId(pub String);

impl Display for ServiceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Exponential weighted moving average of latency
    pub ewma: Duration,
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Active in-flight requests.
    pub active: u32,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct HealthStatus {
    /// Whether the upstream is considered healthy or not.
    pub healthy: bool,
}

#[derive(Debug, Clone)]
pub struct HealthCheckParams {
    pub enable: bool,
    pub failure_threshold: u32,
    pub unhealthy_cooldown: Duration,
}
