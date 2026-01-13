use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct StaticFilesSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub routes: Vec<StaticRouteSpec>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct StaticRouteSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub path: String,
    pub file_dir: PathBuf,
    pub index: Option<String>,
    pub directory_listing: bool,
    pub max_file_size: u64,
    pub compression: CompressionOptsSpec,
    pub cache_policy: CachePolicySpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionOptsSpec {
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl Default for CompressionOptsSpec {
    fn default() -> Self {
        Self {
            small_file_threshold: 256 * 1024, // 256 KiB
            min_gzip_size: 1024,              // 1 KiB
            min_brotli_size: 4 * 1024,        // 4 KiB
            enable_gzip: true,
            enable_brotli: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachePolicySpec {
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}

impl Default for CachePolicySpec {
    fn default() -> Self {
        Self {
            max_age_seconds: 3600,
            public: true,
            immutable: false,
        }
    }
}
