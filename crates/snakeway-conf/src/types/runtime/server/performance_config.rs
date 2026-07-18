use crate::types::PerformanceSpec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = PerformanceSpec)]
pub struct PerformanceConfig {
    /// Enable work stealing between threads.
    pub work_stealing: bool,
    /// Number of parallel accept tasks per listener.
    #[confval(lower(from = parallel_accepts_per_listener, with = narrow::opt_i64_to_usize))]
    pub parallel_accepts_per_listener: Option<usize>,
}
