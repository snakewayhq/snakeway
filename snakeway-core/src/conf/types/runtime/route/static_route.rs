use crate::conf::types::{CachePolicySpec, CompressionOptsSpec, StaticRouteSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct StaticRouteConfig {
    /// The listener this route is attached to.
    pub(crate) listener: String,

    /// Host names allowed to access this route.
    pub(crate) hosts: Vec<String>,

    /// Path prefix (longest-prefix match).
    pub(crate) path: String,

    pub(crate) file_dir: PathBuf,

    pub(crate) index: Option<String>,

    pub(crate) directory_listing: bool,

    pub(crate) max_file_size: u64,

    pub(crate) static_config: CompressionOptions,
    pub(crate) cache_policy: CachePolicy,
}

impl StaticRouteConfig {
    pub(crate) fn new(listener: &str, spec: StaticRouteSpec) -> Self {
        Self {
            listener: listener.to_string(),
            hosts: spec.hosts,
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
pub(crate) struct CompressionOptions {
    pub(crate) small_file_threshold: u64,
    pub(crate) min_gzip_size: u64,
    pub(crate) min_brotli_size: u64,
    pub(crate) enable_gzip: bool,
    pub(crate) enable_brotli: bool,
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
pub(crate) struct CachePolicy {
    pub(crate) max_age_seconds: u32,
    pub(crate) public: bool,
    pub(crate) immutable: bool,
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
