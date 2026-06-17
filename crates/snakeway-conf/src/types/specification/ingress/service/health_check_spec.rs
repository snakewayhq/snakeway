use crate::types::HclInt;
use confval::prelude::Located;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct HealthCheckSpec {
    pub enable: Located<bool>,
    #[confval(default = 3)]
    pub failure_threshold: Located<HclInt>,
    #[confval(default = 10)]
    pub unhealthy_cooldown_seconds: Located<HclInt>,
}

impl Default for HealthCheckSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            failure_threshold: Located::detached(3),
            unhealthy_cooldown_seconds: Located::detached(10),
        }
    }
}
