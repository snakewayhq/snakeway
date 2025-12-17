use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum RouteKind {
    /// Forward request to upstream
    Proxy {
        upstream: String,
    },

    /// Serve files from the local filesystem
    Static {
        path: String,
        file_dir: PathBuf,
        index: bool,
    },
}
