use crate::execution::traffic::HealthStatus;
use crate::execution::traffic::circuit::{CircuitBreakerParams, CircuitState};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct AdminUpstreamView {
    pub(crate) health: HealthStatus,
    pub(crate) circuit: CircuitState,
    pub(crate) active_requests: u32,
    pub(crate) total_requests: u32,
    pub(crate) total_successes: u32,
    pub(crate) total_failures: u32,
    pub(crate) circuit_params: Option<CircuitBreakerParamsView>,
    pub(crate) circuit_details: Option<CircuitBreakerDetailsView>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CircuitBreakerParamsView {
    pub(crate) enabled: bool,
    pub(crate) failure_threshold: u32,
    pub(crate) open_duration_milliseconds: u64,
    pub(crate) half_open_max_requests: u32,
    pub(crate) success_threshold: u32,
    pub(crate) count_http_5xx_as_failure: bool,
}

impl From<&CircuitBreakerParams> for CircuitBreakerParamsView {
    fn from(p: &CircuitBreakerParams) -> Self {
        Self {
            enabled: p.enable_auto_recovery,
            failure_threshold: p.failure_threshold,
            open_duration_milliseconds: p.open_duration.as_millis() as u64,
            half_open_max_requests: p.half_open_max_requests,
            success_threshold: p.success_threshold,
            count_http_5xx_as_failure: p.count_http_5xx_as_failure,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CircuitBreakerDetailsView {
    pub(crate) consecutive_failures: u32,
    pub(crate) opened_at_rfc3339: Option<String>,
    pub(crate) half_open_in_flight: u32,
    pub(crate) half_open_successes: u32,
}
