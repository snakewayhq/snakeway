use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct ConnectionRateLimitingFilterSpec {
    pub(crate) max_connections_per_second: u16,
    pub(crate) window_seconds: u16,
}
