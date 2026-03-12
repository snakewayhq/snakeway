use crate::types::RequestRateLimitingDeviceSpec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceConfig {
    pub enable: bool,
    pub reaction_interval: Duration,
    pub max_requests_per_second: f64,
}

impl From<RequestRateLimitingDeviceSpec> for RequestRateLimitingDeviceConfig {
    fn from(spec: RequestRateLimitingDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            reaction_interval: Duration::from_secs(spec.window_seconds as u64),
            max_requests_per_second: spec.max_requests_per_second as f64,
        }
    }
}
