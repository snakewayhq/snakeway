use crate::types::{HclInt, HclOrigin};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct StaticRouteSpec {
    #[serde(skip)]
    pub origin: HclOrigin,
    pub hosts: Vec<String>,
    pub path: String,
    pub file_dir: PathBuf,
    pub index: Option<String>,
    pub directory_listing: bool,
    pub max_file_size: HclInt,
    pub compression: CompressionOptsSpec,
    pub cache_policy: CachePolicySpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionOptsSpec {
    pub small_file_threshold: HclInt,
    pub min_gzip_size: HclInt,
    pub min_brotli_size: HclInt,
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
    pub max_age_seconds: HclInt,
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
