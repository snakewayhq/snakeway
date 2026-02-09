use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionRateLimiterFilterSpec {
    pub max_connections_per_second: u16,
    pub reaction_interval_in_seconds: u16,
}
