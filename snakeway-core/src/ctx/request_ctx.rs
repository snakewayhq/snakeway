use http::{Extensions, HeaderMap, Method, Uri};
use std::net::{IpAddr, SocketAddr};

/// Canonical request context passed through the Snakeway pipeline
#[derive(Debug)]
pub struct RequestCtx {
    /// HTTP method (immutable)
    pub method: Method,

    /// Original URI as received from the client (immutable, for logging/debugging)
    pub original_uri: Uri,

    /// Path used for routing decisions (mutable by devices before routing)
    pub route_path: String,

    /// Optional override for the upstream request path
    pub upstream_path: Option<String>,

    /// Headers (mutable by devices)
    pub headers: HeaderMap,

    /// Remote IP of the TCP connection (authoritative)
    pub peer_ip: IpAddr,

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default)
    pub extensions: Extensions,

    #[allow(dead_code)]
    /// Request body
    pub body: Vec<u8>,
}

impl RequestCtx {
    pub fn new(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        peer_ip: IpAddr,
        body: Vec<u8>,
    ) -> Self {
        let route_path = uri.path().to_string();

        Self {
            method,
            original_uri: uri,
            route_path,
            upstream_path: None,
            headers,
            peer_ip,
            extensions: Extensions::new(),
            body,
        }
    }

    /// Path used when proxying upstream
    pub fn upstream_path(&self) -> &str {
        self.upstream_path.as_deref().unwrap_or(&self.route_path)
    }
}
