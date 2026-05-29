use crate::types::HclInt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CircuitBreakerSpec {
    /// Enable circuit breaking auto recovery for this service.
    #[serde(default)]
    pub enable_auto_recovery: bool,

    /// Failures in the "closed" state before opening the circuit.
    #[serde(default = "cb_default_failure_threshold")]
    pub failure_threshold: HclInt,

    /// How long to keep the circuit open before allowing probes.
    #[serde(default = "cb_default_open_duration_milliseconds")]
    pub open_duration_milliseconds: HclInt,

    /// How many simultaneous probe requests are allowed in half-open.
    /// (Start with 1; keep it simple and safe.)
    #[serde(default = "cb_default_half_open_max_requests")]
    pub half_open_max_requests: HclInt,

    /// How many successful probes close the circuit again.
    #[serde(default = "cb_default_success_threshold")]
    pub success_threshold: HclInt,

    /// Whether HTTP 5xx responses count as failures for the circuit.
    #[serde(default = "cb_default_count_http_5xx_as_failure")]
    pub count_http_5xx_as_failure: bool,
}

fn cb_default_failure_threshold() -> HclInt {
    5
}
fn cb_default_open_duration_milliseconds() -> HclInt {
    10_000
}
fn cb_default_half_open_max_requests() -> HclInt {
    1
}
fn cb_default_success_threshold() -> HclInt {
    2
}
fn cb_default_count_http_5xx_as_failure() -> bool {
    true
}
