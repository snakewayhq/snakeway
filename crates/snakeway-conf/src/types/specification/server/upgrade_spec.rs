use crate::types::HclInt;
use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use serde::Serialize;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpgradeSpec {
    /// Path to the Unix domain socket used for zero-drop upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<Located<String>>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<Located<HclInt>>,
}

impl Validate for UpgradeSpec {
    fn validate(&self, report: &mut Report) {
        todo!()
    }
}
