use crate::traffic_management::HealthStatus;
use crate::traffic_management::circuit::{CircuitBreakerParams, CircuitState};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AdminUpstreamView {
    pub health: HealthStatus,
    pub circuit: CircuitState,
    pub active_requests: u32,
    pub total_requests: u32,
    pub total_successes: u32,
    pub total_failures: u32,
    pub circuit_params: Option<CircuitBreakerParamsView>,
    pub circuit_details: Option<CircuitBreakerDetailsView>,
}

#[derive(Debug, Serialize)]
pub struct CircuitBreakerParamsView {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub open_duration_milliseconds: u64,
    pub half_open_max_requests: u32,
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
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
pub struct CircuitBreakerDetailsView {
    pub consecutive_failures: u32,
    pub opened_at_rfc3339: Option<String>,
    pub half_open_in_flight: u32,
    pub half_open_successes: u32,
}
