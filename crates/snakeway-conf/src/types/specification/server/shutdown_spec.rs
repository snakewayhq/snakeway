use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(DRAIN_SECONDS, i64, min: 0, max: 300, units: "seconds");
range_constraint!(FORCE_TIMEOUT_SECONDS, i64, min: 1, max: 300, units: "seconds");

#[derive(Debug, Serialize, confval::Spec)]
pub struct ShutdownSpec {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[confval(default = 10)]
    pub drain_seconds: Option<Located<i64>>,

    /// Hard ceiling on total shutdown time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_timeout_seconds: Option<Located<i64>>,
}

impl Default for ShutdownSpec {
    fn default() -> Self {
        Self {
            drain_seconds: Some(Located::detached(10)),
            force_timeout_seconds: None,
        }
    }
}

impl Validate for ShutdownSpec {
    fn validate(&self, report: &mut Report) {
        if let Some(drain) = &self.drain_seconds {
            DRAIN_SECONDS.check_located(drain, "drain_seconds", report);
        }
        if let Some(timeout) = &self.force_timeout_seconds {
            FORCE_TIMEOUT_SECONDS.check_located(timeout, "force_timeout_seconds", report);
        }
    }
}
