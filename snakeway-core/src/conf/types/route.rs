use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,

    /// Target route.
    pub kind: RouteKind,

    pub allow_websocket: bool,
    pub ws_idle_timeout_ms: Option<u64>,
    pub ws_max_connections: Option<u64>,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct ParsedRoute {
    /// The explicit type of the route, to ease route validation.
    pub r#type: ParsedRouteType,

    /// The path to the
    pub path: String,

    /// Mutually exclusive with file_dir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    /// Only valid for upstream services.
    #[serde(default)]
    pub allow_websocket: bool,

    /// Only valid for upstream services.
    #[serde(default)]
    pub ws_idle_timeout_ms: Option<u64>,

    /// Only valid for upstream services.
    #[serde(default)]
    pub ws_max_connections: Option<u64>,

    /// Mutually exclusive with service (validated later)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_dir: Option<String>,

    /// Only valid for static routes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,

    /// Only valid for static routes
    #[serde(default)]
    pub directory_listing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParsedRouteType {
    Service,
    Static,
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
