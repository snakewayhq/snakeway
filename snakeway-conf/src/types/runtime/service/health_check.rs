use crate::types::HealthCheckSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckConfig {
    pub enable: bool,
    pub failure_threshold: u32,
    pub unhealthy_cooldown_seconds: u64,
}

impl From<HealthCheckSpec> for HealthCheckConfig {
    fn from(spec: HealthCheckSpec) -> Self {
        Self {
            enable: spec.enable,
            failure_threshold: spec.failure_threshold,
            unhealthy_cooldown_seconds: spec.unhealthy_cooldown_seconds,
        }
    }
}
