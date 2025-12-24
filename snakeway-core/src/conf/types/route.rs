use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RouteConfig {
    /// Path prefix (longest-prefix match)
    pub path: String,

    /// Target service (mutually exclusive with file_dir)
    pub service: Option<String>,

    /// Static file serving
    pub file_dir: Option<String>,

    /// Optional index file for directories
    pub index: Option<String>,

    /// Enable directory listing
    #[serde(default)]
    pub directory_listing: bool,
}
