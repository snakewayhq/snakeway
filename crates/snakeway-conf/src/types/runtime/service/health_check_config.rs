use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckConfig {
    pub enable: bool,
    pub failure_threshold: u64,
    pub unhealthy_cooldown_seconds: u64,
}
