use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceConfig {
    pub enable: bool,
    pub reaction_interval: Duration,
    pub max_requests_per_second: f64,
}
