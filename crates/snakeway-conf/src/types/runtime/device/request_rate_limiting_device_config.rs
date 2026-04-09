use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceConfig {
    pub enable: bool,
    pub reaction_interval: Duration,
    pub max_requests_per_second: f64,
    pub paths: SmallVec<[String; 4]>,
}
