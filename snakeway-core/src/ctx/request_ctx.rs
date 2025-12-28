use crate::server::UpstreamId;
use crate::traffic::ServiceId;
use http::{Extensions, HeaderMap, Method, Uri};
use std::net::IpAddr;

/// Canonical request context passed through the Snakeway pipeline
#[derive(Debug)]
pub struct RequestCtx {
    /// Service name for routing decisions.
    pub service: Option<String>,

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

    /// Is it a websocket upgrade request (or not)?
    pub is_upgrade_req: bool,

    /// Was a websocket connection opened?
    pub ws_opened: bool,

    /// Is it a gRPC request?
    pub is_grpc: bool,

    /// Upstream authority for HTTP/2 requests.
    pub upstream_authority: Option<String>,

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default).
    pub extensions: Extensions,

    pub selected_upstream: Option<(ServiceId, UpstreamId)>,

    #[allow(dead_code)]
    /// Request body
    pub body: Vec<u8>,
}

impl RequestCtx {
    pub fn new(
        service: Option<String>,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        peer_ip: IpAddr,
        is_upgrade_req: bool,
        body: Vec<u8>,
    ) -> Self {
        let route_path = uri.path().to_string();

        Self {
            service,
            method,
            original_uri: uri,
            route_path,
            upstream_path: None,
            headers,
            peer_ip,
            is_upgrade_req,
            ws_opened: false,
            is_grpc: false,
            upstream_authority: None,
            extensions: Extensions::new(),
            selected_upstream: None,
            body,
        }
    }

    /// Path used when proxying upstream
    pub fn upstream_path(&self) -> &str {
        self.upstream_path.as_deref().unwrap_or(&self.route_path)
    }

    /// Returns the upstream authority (host:port) to use for HTTP/2 requests.
    ///
    /// This is typically set when proxying to HTTP/2 backends that require
    /// a specific :authority pseudo-header value.
    pub fn upstream_authority(&self) -> Option<&str> {
        self.upstream_authority.as_deref()
    }
}
