use crate::types::HclInt;
use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
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

impl Validate for HealthCheckSpec {
    fn validate(&self, report: &mut Report) {
        todo!()
    }
}
