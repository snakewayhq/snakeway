use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CircuitBreakerConfig {
    pub enable_auto_recovery: bool,
    pub failure_threshold: u32,
    pub open_duration_milliseconds: u64,
    pub half_open_max_requests: u32,
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}
