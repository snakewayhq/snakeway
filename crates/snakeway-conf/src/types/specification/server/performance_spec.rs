use crate::types::HclInt;
use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use serde::Serialize;

#[derive(Debug, Serialize, confval::Spec)]
pub struct PerformanceSpec {
    #[confval(default = true)]
    pub work_stealing: Located<bool>,

    /// Number of parallel accept tasks per listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_accepts_per_listener: Option<Located<HclInt>>,
}

impl Default for PerformanceSpec {
    fn default() -> Self {
        Self {
            work_stealing: Located::detached(true),
            parallel_accepts_per_listener: None,
        }
    }
}

impl Validate for PerformanceSpec {
    fn validate(&self, report: &mut Report) {
        todo!()
    }
}
