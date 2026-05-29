use crate::types::HclInt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionRateLimitingFilterSpec {
    pub max_connections_per_second: HclInt,
    pub window_seconds: HclInt,
}
