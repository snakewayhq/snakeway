use crate::types::{HclInt, HclOrigin};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

    pub enable: bool,
    pub max_requests_per_second: HclInt,
    pub window_seconds: HclInt,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[serde(default)]
    pub paths: Vec<String>,
}
