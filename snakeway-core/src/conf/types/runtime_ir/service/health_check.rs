use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckConfig {
    pub enable: bool,
    #[serde(default = "hc_default_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "hc_default_unhealthy_cooldown_seconds")]
    pub unhealthy_cooldown_seconds: u64,
}

fn hc_default_threshold() -> u32 {
    3
}

fn hc_default_unhealthy_cooldown_seconds() -> u64 {
    10
}
