use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RequestRateLimitingDeviceSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,

    pub(crate) enable: bool,
    pub(crate) max_requests_per_second: u16,
    pub(crate) window_seconds: u16,
}
