use crate::conf::types::RateLimiterFilterSpec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct RateLimiterFilterConfig {
    pub max_connections_per_second: f64,
    pub reaction_interval: Duration,
}

impl From<RateLimiterFilterSpec> for RateLimiterFilterConfig {
    fn from(spec: RateLimiterFilterSpec) -> Self {
        Self {
            max_connections_per_second: spec.max_connections_per_second as f64,
            reaction_interval: Duration::from_secs(spec.reaction_interval_in_seconds as u64),
        }
    }
}
