use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticRouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,
    pub file_dir: PathBuf,

    pub index: Option<String>,

    pub directory_listing: bool,

    pub static_config: StaticFileConfig,
    pub cache_policy: StaticCachePolicy,
    pub listener: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticFileConfig {
    pub max_file_size: u64,
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticCachePolicy {
    pub max_age_secs: u32,
    pub public: bool,
    pub immutable: bool,
}
