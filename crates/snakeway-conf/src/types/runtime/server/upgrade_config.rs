use crate::types::UpgradeSpec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = UpgradeSpec)]
pub struct UpgradeConfig {
    /// Path to the Unix domain socket used for zero-drop upgrades (FD transfer).
    pub sock: Option<String>,
    /// Maximum retries when connecting/accepting on the upgrade socket.
    #[confval(lower(from = max_retries, with = narrow::opt_i64_to_usize))]
    pub max_retries: Option<usize>,
}
