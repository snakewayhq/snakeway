use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
pub struct ConnectionRateLimitingFilterConfig {
    pub max_connections_per_second: f64,
    pub reaction_interval: Duration,
}
