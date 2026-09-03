use confval::prelude::{Located, Report, Validate, range_constraint};
use serde::Serialize;

range_constraint!(DRAIN_SECONDS, i64, min: 0, max: 300, units: "seconds");
range_constraint!(FORCE_TIMEOUT_SECONDS, i64, min: 1, max: 300, units: "seconds");

#[derive(Debug, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct ShutdownSpec {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[confval(default = 10, range = DRAIN_SECONDS)]
    pub drain_seconds: Option<Located<i64>>,

    /// Hard ceiling on total shutdown time.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = FORCE_TIMEOUT_SECONDS)]
    pub force_timeout_seconds: Option<Located<i64>>,
}

impl Validate for ShutdownSpec {
    fn validate(&self, _report: &mut Report) {}
}
