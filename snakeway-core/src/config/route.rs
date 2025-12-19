use crate::config::static_files::StaticFileConfig;
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

    /// Static file compression configuration.
    #[serde(default)]
    pub config: StaticFileConfig,
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
