use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionRateLimitingFilterSpec {
    pub max_connections_per_second: u16,
    pub window_seconds: u16,
}
