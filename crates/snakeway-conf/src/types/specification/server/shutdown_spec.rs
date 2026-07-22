use crate::types::HclInt;
use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use serde::Serialize;

#[derive(Debug, Serialize, confval::Spec)]
pub struct ShutdownSpec {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[confval(default = 10)]
    pub drain_seconds: Option<Located<HclInt>>,

    /// Hard ceiling on total shutdown time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_timeout_seconds: Option<Located<HclInt>>,
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
        todo!()
    }
}
