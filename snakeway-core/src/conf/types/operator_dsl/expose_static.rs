use crate::conf::types::TlsConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct ExposeStaticConfig {
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub routes: Vec<ExposeStaticRouteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeStaticRouteConfig {
    pub path: String,
    pub file_dir: PathBuf,
    pub index: Option<String>,
    pub directory_listing: bool,
    pub max_file_size: u64,
    pub compression: CompressionConfig,
    pub cache_policy: CachePolicyConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl Default for CompressionConfig {
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
pub struct CachePolicyConfig {
    pub max_age_secs: u32,
    pub public: bool,
    pub immutable: bool,
}

impl Default for CachePolicyConfig {
    fn default() -> Self {
        Self {
            max_age_secs: 3600,
            public: true,
            immutable: false,
        }
    }
}
