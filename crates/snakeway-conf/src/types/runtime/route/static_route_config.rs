use crate::types::{CachePolicySpec, CompressionOptsSpec, StaticRouteSpec};
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
    #[map(~ as u64)]
    pub small_file_threshold: u64,
    #[map(~ as u64)]
    pub min_gzip_size: u64,
    #[map(~ as u64)]
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(CachePolicySpec)]
pub struct CachePolicy {
    #[map(~ as u32)]
    pub max_age_seconds: u32,
    pub public: bool,
    pub immutable: bool,
}

impl StaticRouteConfig {
    pub fn new(listener: &str, spec: StaticRouteSpec) -> Self {
        Self {
            listener: listener.to_string(),
            hosts: spec.hosts,
            path: spec.path,
            file_dir: spec.file_dir,
            index: spec.index,
            directory_listing: spec.directory_listing,
            max_file_size: spec.max_file_size as u64,
            static_config: spec.compression.into(),
            cache_policy: spec.cache_policy.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HclOrigin;

    #[test]
    fn new_maps_fields_correctly() {
        // Arrange
        let spec = StaticRouteSpec {
            origin: HclOrigin::default(),
            hosts: vec!["static.example.com".to_string()],
            path: "/assets".to_string(),
            file_dir: PathBuf::from("/var/www/static"),
            index: Some("index.html".to_string()),
            directory_listing: true,
            max_file_size: 10_000_000,
            compression: CompressionOptsSpec::default(),
            cache_policy: CachePolicySpec::default(),
        };

        // Act
        let config = StaticRouteConfig::new("my-listener", spec);

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
