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

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default)
    pub extensions: Extensions,

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
            extensions: Extensions::new(),
            body,
        }
    }

    /// Path used when proxying upstream
    pub fn upstream_path(&self) -> &str {
        self.upstream_path.as_deref().unwrap_or(&self.route_path)
    }
}
