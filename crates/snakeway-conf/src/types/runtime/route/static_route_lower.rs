use crate::types::StaticRouteSpec;

use super::StaticRouteConfig;

impl StaticRouteConfig {
    pub fn new(listener: &str, spec: StaticRouteSpec) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CachePolicySpec, CompressionOptsSpec, HclOrigin};
    use std::path::PathBuf;

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
