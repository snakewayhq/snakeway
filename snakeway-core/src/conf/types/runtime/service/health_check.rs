use crate::conf::types::HealthCheckSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct HealthCheckConfig {
    pub(crate) enable: bool,
    pub(crate) failure_threshold: u32,
    pub(crate) unhealthy_cooldown_seconds: u64,
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
