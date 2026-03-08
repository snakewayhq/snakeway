use crate::conf::types::CircuitBreakerSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct CircuitBreakerConfig {
    pub(crate) enable_auto_recovery: bool,
    pub(crate) failure_threshold: u32,
    pub(crate) open_duration_milliseconds: u64,
    pub(crate) half_open_max_requests: u32,
    pub(crate) success_threshold: u32,
    pub(crate) count_http_5xx_as_failure: bool,
}

impl From<CircuitBreakerSpec> for CircuitBreakerConfig {
    fn from(spec: CircuitBreakerSpec) -> Self {
        Self {
            enable_auto_recovery: spec.enable_auto_recovery,
            failure_threshold: spec.failure_threshold,
            open_duration_milliseconds: spec.open_duration_milliseconds,
            half_open_max_requests: spec.half_open_max_requests,
            success_threshold: spec.success_threshold,
            count_http_5xx_as_failure: spec.count_http_5xx_as_failure,
        }
    }
}
