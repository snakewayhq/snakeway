use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct RateLimiterFilterSpec {
    pub max_connections_per_second: f64,
    pub reaction_interval_in_seconds: u64,
}
