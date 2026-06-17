use crate::types::{CachePolicySpec, CompressionOptsSpec, StaticRouteSpec};
use confval::prelude::{Lower, Report, narrow};
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

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = CompressionOptsSpec)]
pub struct CompressionOptions {
    #[confval(lower(from = small_file_threshold, with = narrow::i64_to_u64))]
    pub small_file_threshold: u64,
    #[confval(lower(from = min_gzip_size, with = narrow::i64_to_u64))]
    pub min_gzip_size: u64,
    #[confval(lower(from = min_brotli_size, with = narrow::i64_to_u64))]
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = CachePolicySpec)]
pub struct CachePolicy {
    #[confval(lower(from = max_age_seconds, with = narrow::i64_to_u32))]
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}

impl StaticRouteConfig {
    pub fn new(listener: &str, spec: &StaticRouteSpec, report: &mut Report) -> Option<Self> {
        Some(Self {
            listener: listener.to_string(),
            hosts: spec.hosts.iter().map(|h| h.value.clone()).collect(),
            path: spec.path.value.clone(),
            file_dir: spec.file_dir.value.clone(),
            index: spec.index.as_ref().map(|i| i.value.clone()),
            directory_listing: spec.directory_listing.value,
            max_file_size: narrow::i64_to_u64(&spec.max_file_size, report)?,
            static_config: CompressionOptions::lower(&spec.compression.value, report)?,
            cache_policy: CachePolicy::lower(&spec.cache_policy.value, report)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_maps_fields_correctly() {
        // Arrange
        use confval::prelude::Located;
        let spec = StaticRouteSpec {
            hosts: vec![Located::detached("static.example.com".to_string())],
            path: Located::detached("/assets".to_string()),
            file_dir: Located::detached(PathBuf::from("/var/www/static")),
            index: Some(Located::detached("index.html".to_string())),
            directory_listing: Located::detached(true),
            max_file_size: Located::detached(10_000_000),
            compression: Located::detached(CompressionOptsSpec::default()),
            cache_policy: Located::detached(CachePolicySpec::default()),
        };

        // Act
        let config = StaticRouteConfig::new("my-listener", &spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.listener, "my-listener");
        assert_eq!(config.hosts, vec!["static.example.com"]);
        assert_eq!(config.path, "/assets");
        assert_eq!(config.file_dir, PathBuf::from("/var/www/static"));
        assert_eq!(config.index, Some("index.html".to_string()));
        assert!(config.directory_listing);
        assert_eq!(config.max_file_size, 10_000_000);
    }
}
