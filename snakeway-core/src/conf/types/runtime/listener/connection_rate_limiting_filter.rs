use crate::conf::types::ConnectionRateLimitingFilterSpec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct ConnectionRateLimitingFilterConfig {
    pub(crate) max_connections_per_second: f64,
    pub(crate) reaction_interval: Duration,
}

impl From<ConnectionRateLimitingFilterSpec> for ConnectionRateLimitingFilterConfig {
    fn from(spec: ConnectionRateLimitingFilterSpec) -> Self {
        Self {
            max_connections_per_second: spec.max_connections_per_second as f64,
            reaction_interval: Duration::from_secs(spec.window_seconds as u64),
        }
    }
}
