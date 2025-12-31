use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// Load balancing strategy
    #[serde(default = "default_strategy")]
    pub strategy: LoadBalancingStrategy,

    #[serde(default)]
    pub upstream: Vec<UpstreamConfig>,

    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,

    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckConfig {
    #[serde(default = "hc_default_enable")]
    pub enable: bool,
    #[serde(default = "hc_default_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "hc_default_unhealthy_cooldown_seconds")]
    pub unhealthy_cooldown_seconds: u64,
}

fn hc_default_enable() -> bool {
    true
}

fn hc_default_threshold() -> u32 {
    3
}

fn hc_default_unhealthy_cooldown_seconds() -> u64 {
    10
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaking auto recovery for this service.
    #[serde(default = "cb_default_enable_auto_recovery")]
    pub enable_auto_recovery: bool,

    /// Failures in the "closed" state before opening the circuit.
    #[serde(default = "cb_default_failure_threshold")]
    pub failure_threshold: u32,

    /// How long to keep the circuit open before allowing probes.
    #[serde(default = "cb_default_open_duration_ms")]
    pub open_duration_ms: u64,

    /// How many simultaneous probe requests are allowed in half-open.
    /// (Start with 1; keep it simple and safe.)
    #[serde(default = "cb_default_half_open_max_requests")]
    pub half_open_max_requests: u32,

    /// How many successful probes close the circuit again.
    #[serde(default = "cb_default_success_threshold")]
    pub success_threshold: u32,

    /// Whether HTTP 5xx responses count as failures for the circuit.
    #[serde(default = "cb_default_count_http_5xx_as_failure")]
    pub count_http_5xx_as_failure: bool,
}

fn cb_default_enable_auto_recovery() -> bool {
    false
}
fn cb_default_failure_threshold() -> u32 {
    5
}
fn cb_default_open_duration_ms() -> u64 {
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

fn default_strategy() -> LoadBalancingStrategy {
    LoadBalancingStrategy::Failover
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub url: String,

    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}
