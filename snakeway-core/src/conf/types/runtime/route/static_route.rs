use crate::conf::types::{CachePolicySpec, CompressionOptsSpec, StaticRouteSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticRouteConfig {
    /// The listener this route is attached to.
    pub listener: String,

    /// Path prefix (longest-prefix match).
    pub path: String,
    pub file_dir: PathBuf,

    pub index: Option<String>,

    pub directory_listing: bool,

    pub max_file_size: u64,

    pub static_config: CompressionOptions,
    pub cache_policy: CachePolicy,
}

impl StaticRouteConfig {
    pub fn new(listener: &str, spec: StaticRouteSpec) -> Self {
        Self {
            listener: listener.to_string(),
            path: spec.path,
            file_dir: spec.file_dir,
            index: spec.index,
            directory_listing: spec.directory_listing,
            max_file_size: spec.max_file_size,
            static_config: spec.compression.into(),
            cache_policy: spec.cache_policy.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionOptions {
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl From<CompressionOptsSpec> for CompressionOptions {
    fn from(spec: CompressionOptsSpec) -> Self {
        Self {
            small_file_threshold: spec.small_file_threshold,
            min_gzip_size: spec.min_gzip_size,
            min_brotli_size: spec.min_brotli_size,
            enable_gzip: spec.enable_gzip,
            enable_brotli: spec.enable_brotli,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachePolicy {
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}

impl From<CachePolicySpec> for CachePolicy {
    fn from(spec: CachePolicySpec) -> Self {
        Self {
            max_age_seconds: spec.max_age_seconds,
            public: spec.public,
            immutable: spec.immutable,
        }
    }
}
