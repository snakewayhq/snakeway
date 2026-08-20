use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(PARALLEL_ACCEPTS_PER_LISTENER, i64, min: 1, max: 64);

#[derive(Debug, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct PerformanceSpec {
    #[confval(default = true)]
    pub work_stealing: Located<bool>,

    /// Number of parallel accept tasks per listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = PARALLEL_ACCEPTS_PER_LISTENER)]
    pub parallel_accepts_per_listener: Option<Located<i64>>,
}

impl Validate for PerformanceSpec {
    fn validate(&self, _report: &mut Report) {}
}
