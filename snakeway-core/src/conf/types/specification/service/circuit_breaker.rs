use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct CircuitBreakerSpec {
    /// Enable circuit breaking auto recovery for this service.
    #[serde(default)]
    pub(crate) enable_auto_recovery: bool,

    /// Failures in the "closed" state before opening the circuit.
    #[serde(default = "cb_default_failure_threshold")]
    pub(crate) failure_threshold: u32,

    /// How long to keep the circuit open before allowing probes.
    #[serde(default = "cb_default_open_duration_milliseconds")]
    pub(crate) open_duration_milliseconds: u64,

    /// How many simultaneous probe requests are allowed in half-open.
    /// (Start with 1; keep it simple and safe.)
    #[serde(default = "cb_default_half_open_max_requests")]
    pub(crate) half_open_max_requests: u32,

    /// How many successful probes close the circuit again.
    #[serde(default = "cb_default_success_threshold")]
    pub(crate) success_threshold: u32,

    /// Whether HTTP 5xx responses count as failures for the circuit.
    #[serde(default = "cb_default_count_http_5xx_as_failure")]
    pub(crate) count_http_5xx_as_failure: bool,
}

fn cb_default_failure_threshold() -> u32 {
    5
}
fn cb_default_open_duration_milliseconds() -> u64 {
    10_000
}
fn cb_default_half_open_max_requests() -> u32 {
    1
}
fn cb_default_success_threshold() -> u32 {
    2
}
fn cb_default_count_http_5xx_as_failure() -> bool {
    true
}
