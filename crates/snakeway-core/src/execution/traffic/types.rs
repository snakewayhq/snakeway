use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize)]
pub(crate) struct ServiceId(pub(crate) Arc<str>);

impl Display for ServiceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LatencyStats {
    /// Exponential weighted moving average of latency
    pub(crate) ewma: Duration,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub(crate) struct HealthStatus {
    /// Whether the upstream is considered healthy or not.
    pub(crate) healthy: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct HealthCheckParams {
    pub(crate) enable: bool,
    pub(crate) failure_threshold: u64,
    pub(crate) unhealthy_cooldown: Duration,
}
