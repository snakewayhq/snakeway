use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

    pub enable: bool,
    pub max_requests_per_second: u16,
    pub window_seconds: u16,
}
