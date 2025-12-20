use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    /// URL path prefix, e.g. "/", "/static"
    pub path: String,

    /// Proxy upstream (mutually exclusive with file_dir)
    pub upstream: Option<String>,

    /// Local directory for static files (mutually exclusive with upstream)
    pub file_dir: Option<String>,

    /// Whether to serve index.html for directories
    #[serde(default)]
    pub index: bool,

    #[serde(default)]
    pub directory_listing: bool,

    pub directory_listing_format: Option<String>,

    /// Static file streaming and compression configuration
    #[serde(default)]
    pub static_config: StaticFileConfig,

    /// Cache policy for static files
    #[serde(default)]
    pub cache_policy: StaticCachePolicy,
}

impl RouteConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        match (&self.upstream, &self.file_dir) {
            (Some(_), None) => Ok(()),
            (None, Some(_)) => Ok(()),
            _ => anyhow::bail!(
                "route '{}' must define exactly one of `upstream` or `file_dir`",
                self.path
            ),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
