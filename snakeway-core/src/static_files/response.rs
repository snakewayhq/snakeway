use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use tokio::fs;

#[derive(Debug)]
pub enum ServeError {
    NotFound,
    Forbidden,
    Io,
}

pub enum StaticBody {
    Empty,
    /// Useful for tiny files/errors.
    Bytes(Bytes),
    /// Useful for large files that require streaming from disk.
    File(fs::File),
}

pub struct StaticResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: StaticBody,
}

/// Conditional request headers for cache validation and content negotiation
#[derive(Debug, Default)]
pub struct ConditionalHeaders {
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
    pub accept_encoding: Option<String>,
}
