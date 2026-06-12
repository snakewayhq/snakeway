use crate::types::{CachePolicySpec, CompressionOptsSpec, StaticRouteSpec};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionOptions {
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl From<&CompressionOptsSpec> for CompressionOptions {
    fn from(spec: &CompressionOptsSpec) -> Self {
        Self {
            small_file_threshold: spec.small_file_threshold.value as u64,
            min_gzip_size: spec.min_gzip_size.value as u64,
            min_brotli_size: spec.min_brotli_size.value as u64,
            enable_gzip: spec.enable_gzip.value,
            enable_brotli: spec.enable_brotli.value,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachePolicy {
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}

impl From<&CachePolicySpec> for CachePolicy {
    fn from(spec: &CachePolicySpec) -> Self {
        Self {
            max_age_seconds: spec.max_age_seconds.value as u32,
            public: spec.public.value,
            immutable: spec.immutable.value,
        }
    }
}

impl StaticRouteConfig {
    pub fn new(listener: &str, spec: &StaticRouteSpec) -> Self {
        Self {
            listener: listener.to_string(),
            hosts: spec.hosts.iter().map(|h| h.value.clone()).collect(),
            path: spec.path.value.clone(),
            file_dir: spec.file_dir.value.clone(),
            index: spec.index.as_ref().map(|i| i.value.clone()),
            directory_listing: spec.directory_listing.value,
            max_file_size: spec.max_file_size.value as u64,
            static_config: (&spec.compression.value).into(),
            cache_policy: (&spec.cache_policy.value).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_maps_fields_correctly() {
        // Arrange
        use confval::provenance::Located;
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
        let config = StaticRouteConfig::new("my-listener", &spec);

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
