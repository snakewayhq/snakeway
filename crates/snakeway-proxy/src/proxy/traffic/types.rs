use http::{HeaderMap, StatusCode};

/// Declared Content-Length from the request headers, stored in extensions
/// during `request_filter` for comparison at end-of-stream.
#[derive(Debug, Clone, Copy)]
struct DeclaredContentLength(u64);

/// Running total of body bytes received from the downstream client,
/// updated per-chunk in `request_body_filter`.
#[derive(Debug, Clone, Copy)]
struct BodyBytesReceived(u64);

/// Snapshot of the upstream response status and headers, stored in extensions
/// during `upstream_response_filter` for use by `upstream_response_body_filter`.
#[derive(Debug, Clone)]
struct UpstreamResponseSnapshot {
    status: StatusCode,
    headers: HeaderMap,
}
