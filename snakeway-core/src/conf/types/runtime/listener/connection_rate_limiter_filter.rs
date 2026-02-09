use crate::conf::types::ConnectionRateLimiterFilterSpec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionRateLimiterFilterConfig {
    pub max_connections_per_second: f64,
    pub reaction_interval: Duration,
}

impl From<ConnectionRateLimiterFilterSpec> for ConnectionRateLimiterFilterConfig {
    fn from(spec: ConnectionRateLimiterFilterSpec) -> Self {
        Self {
            max_connections_per_second: spec.max_connections_per_second as f64,
            reaction_interval: Duration::from_secs(spec.window_seconds as u64),
        }
    }
}
