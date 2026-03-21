use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use tokio::fs;

#[derive(Debug)]
pub(crate) enum ServeError {
    NotFound,
    Forbidden,
    Io,
}

pub(crate) enum StaticBody {
    Empty,
    /// Useful for tiny files/errors.
    Bytes(Bytes),
    /// Useful for large files that require streaming from disk.
    File(fs::File),

    /// Useful for serving range requests for large (media) files.
    RangedFile {
        file: fs::File,
        remaining: u64,
    },
}

pub(crate) struct StaticResponse {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: StaticBody,
}

/// Conditional request headers for cache validation and content negotiation
#[derive(Debug, Default)]
pub(crate) struct ConditionalHeaders {
    pub(crate) if_none_match: Option<String>,
    pub(crate) if_modified_since: Option<String>,
    pub(crate) accept_encoding: Option<String>,
    pub(crate) range: Option<String>,
}
