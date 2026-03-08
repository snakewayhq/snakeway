use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct StaticFilesSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) routes: Vec<StaticRouteSpec>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct StaticRouteSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) hosts: Vec<String>,
    pub(crate) path: String,
    pub(crate) file_dir: PathBuf,
    pub(crate) index: Option<String>,
    pub(crate) directory_listing: bool,
    pub(crate) max_file_size: u64,
    pub(crate) compression: CompressionOptsSpec,
    pub(crate) cache_policy: CachePolicySpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct CompressionOptsSpec {
    pub(crate) small_file_threshold: u64,
    pub(crate) min_gzip_size: u64,
    pub(crate) min_brotli_size: u64,
    pub(crate) enable_gzip: bool,
    pub(crate) enable_brotli: bool,
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
pub(crate) struct CachePolicySpec {
    pub(crate) max_age_seconds: u32,
    pub(crate) public: bool,
    pub(crate) immutable: bool,
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
