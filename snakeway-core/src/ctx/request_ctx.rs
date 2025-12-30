use crate::server::UpstreamId;
use crate::traffic::{ServiceId, UpstreamOutcome};
use http::{Extensions, HeaderMap, Method, Uri};
use pingora::prelude::Session;
use std::net::{IpAddr, Ipv4Addr};

/// Canonical request context passed through the Snakeway pipeline
#[derive(Debug)]
pub struct RequestCtx {
    /// Lifecycle flag to determine if the context has already been hydrated from a session.
    pub hydrated: bool,

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

    pub upstream_outcome: Option<UpstreamOutcome>,
    pub cb_started: bool,

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
            hydrated: false,
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
            upstream_outcome: None,
            cb_started: false,
            body,
        }
    }

    pub fn empty() -> Self {
        Self {
            // Request lifecycle flag.
            hydrated: false,

            // Request identity and content.
            method: Method::GET,                 // dummy
            original_uri: Uri::from_static("/"), // dummy
            headers: HeaderMap::new(),
            body: vec![],

            // Upstream/routing related.
            route_path: "/".into(),
            service: None,
            selected_upstream: None,
            upstream_path: None,

            // Protocol flags that help figure out what to do with the request.
            is_grpc: false,
            is_upgrade_req: false,
            ws_opened: false,

            // Required for gRPC.
            upstream_authority: None,

            // Traffic/Circuit-breaker.
            cb_started: false,
            upstream_outcome: None,

            // Peer info - Pingora fills this out later.
            peer_ip: Ipv4Addr::UNSPECIFIED.into(),

            // Device related data.
            extensions: Extensions::new(),
        }
    }

    pub fn hydrate_from_session(&mut self, session: &Session) {
        debug_assert!(!self.hydrated, "RequestCtx hydrated twice");

        let req = session.req_header();

        self.method = req.method.clone();
        self.original_uri = req.uri.clone();
        self.headers = req.headers.clone();
        self.route_path = req.uri.path().to_string();
        self.is_upgrade_req = session.is_upgrade_req();
        self.is_grpc = req
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with("application/grpc"));

        self.hydrated = true;
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
