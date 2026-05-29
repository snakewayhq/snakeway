use crate::types::HclInt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckSpec {
    pub enable: bool,
    #[serde(default = "hc_default_threshold")]
    pub failure_threshold: HclInt,
    #[serde(default = "hc_default_unhealthy_cooldown_seconds")]
    pub unhealthy_cooldown_seconds: HclInt,
}

fn hc_default_threshold() -> HclInt {
    3
}

fn hc_default_unhealthy_cooldown_seconds() -> HclInt {
    10
}
