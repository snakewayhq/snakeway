use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct HealthCheckSpec {
    pub(crate) enable: bool,
    #[serde(default = "hc_default_threshold")]
    pub(crate) failure_threshold: u32,
    #[serde(default = "hc_default_unhealthy_cooldown_seconds")]
    pub(crate) unhealthy_cooldown_seconds: u64,
}

fn hc_default_threshold() -> u32 {
    3
}

fn hc_default_unhealthy_cooldown_seconds() -> u64 {
    10
}
