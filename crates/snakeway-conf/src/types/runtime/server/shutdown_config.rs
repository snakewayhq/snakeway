use crate::types::ShutdownSpec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = ShutdownSpec)]
pub struct ShutdownConfig {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[confval(lower(from = drain_seconds, with = narrow::opt_i64_to_u64))]
    pub drain_seconds: Option<u64>,
    /// Hard ceiling on total shutdown time.
    #[confval(lower(from = force_timeout_seconds, with = narrow::opt_i64_to_u64))]
    pub force_timeout_seconds: Option<u64>,
}
