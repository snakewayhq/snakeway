use crate::types::{CachePolicySpec, CompressionOptsSpec};
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticRouteConfig {
    /// The listener this route is attached to.
    pub listener: String,

    /// Host names allowed to access this route.
    pub hosts: Vec<String>,

    /// Path prefix (longest-prefix match).
    pub path: String,

    pub file_dir: PathBuf,

    pub index: Option<String>,

    pub directory_listing: bool,

    pub max_file_size: u64,

    pub static_config: CompressionOptions,
    pub cache_policy: CachePolicy,
}

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(CompressionOptsSpec)]
pub struct CompressionOptions {
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(CachePolicySpec)]
pub struct CachePolicy {
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}
