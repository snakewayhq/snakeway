use std::path::PathBuf;
use crate::config::StaticFileConfig;

#[derive(Debug, Clone)]
pub enum RouteKind {
    /// Forward request to upstream
    Proxy { upstream: String },

    /// Serve files from the local filesystem
    Static {
        path: String,
        file_dir: PathBuf,
        index: bool,
        config: StaticFileConfig,
    },
}
