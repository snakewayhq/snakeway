use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize)]
pub struct ServiceId(pub(crate) Arc<str>);

impl ServiceId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
pub struct HealthStatus {
    /// Whether the upstream is considered healthy or not.
    pub healthy: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct HealthCheckParams {
    pub(crate) enable: bool,
    pub(crate) failure_threshold: u64,
    pub(crate) unhealthy_cooldown: Duration,
}
