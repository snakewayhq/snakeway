use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteKind {
    Service {
        name: String,
    },
    Static {
        dir: String,
        index: Option<String>,
        directory_listing: bool,
        static_config: StaticFileConfig,
        cache_policy: StaticCachePolicy,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticFileConfig {
    pub max_file_size: u64,
    pub small_file_threshold: u64,
    pub min_gzip_size: u64,
    pub min_brotli_size: u64,
    pub enable_gzip: bool,
    pub enable_brotli: bool,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024,  // 10 MiB
            small_file_threshold: 256 * 1024, // 256 KiB
            min_gzip_size: 1024,              // 1 KiB
            min_brotli_size: 4 * 1024,        // 4 KiB
            enable_gzip: true,
            enable_brotli: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticCachePolicy {
    pub max_age: u32, // seconds
    pub public: bool,
    pub immutable: bool,
}

impl Default for StaticCachePolicy {
    fn default() -> Self {
        Self {
            max_age: 3600, // 1 hour
            public: true,
            immutable: false,
        }
    }
}
