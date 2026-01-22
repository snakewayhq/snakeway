use crate::ctx::RequestId;
use crate::ctx::request::NormalizedRequest;
use crate::ctx::request::error::RequestRejectError;
use crate::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode, normalize_headers, normalize_path,
    normalize_query,
};
use crate::route::types::RouteId;
use crate::runtime::UpstreamId;
use crate::traffic_management::{AdmissionGuard, ServiceId, UpstreamOutcome};
use crate::ws_connection_management::WsConnectionGuard;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use pingora::prelude::Session;
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;
use std::net::{IpAddr, Ipv4Addr};

/// Canonical request context passed through the Snakeway pipeline
#[derive(Debug)]
pub struct RequestCtx {
    /// Holds the WS connection slot for the lifetime of the connection
    pub ws_guard: Option<WsConnectionGuard>,

    /// It is necessary to guard requests to ensure proper circuit breaker state updates.
    pub admission_guard: Option<AdmissionGuard>,

    /// Lifecycle flag to determine if the context has already been hydrated from a session.
    pub hydrated: bool,

    /// Service name for routing decisions.
    pub service: Option<String>,

    /// HTTP method (immutable)
    pub method: Option<Method>,

    /// Original URI as received from the client (immutable, for logging/debugging)
    pub original_uri: Option<Uri>,

    pub query_string: Option<String>,

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

    /// HTTP version (immutable)
    pub protocol_version: Option<Version>,

    /// Upstream authority for HTTP/2 requests.
    pub upstream_authority: Option<String>,

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default).
    pub extensions: Extensions,

    /// Normalized request representation for routing and processing.
    pub normalized_request: Option<NormalizedRequest>,

    /// Has the request been normalized?
    pub normalized: bool,

    /// Route ID for routing decisions.
    pub route_id: Option<RouteId>,

    /// Selected upstream and outcome
    pub selected_upstream: Option<(ServiceId, UpstreamId)>,

    /// Outcome of upstream selection
    pub upstream_outcome: Option<UpstreamOutcome>,

    /// Circuit breaker started?
    pub cb_started: bool,

    #[allow(dead_code)]
    /// Request body
    pub body: Vec<u8>,
}

impl Default for RequestCtx {
    fn default() -> Self {
        Self::empty()
    }
}

impl RequestCtx {
    pub fn empty() -> Self {
        Self {
            route_id: None,

            // Request lifecycle-related.
            hydrated: false,
            admission_guard: None,
            ws_guard: None,

            // Request identity and content.
            method: None,
            original_uri: None,
            query_string: None,
            headers: HeaderMap::new(),
            body: vec![],

            // Upstream/routing related.
            route_path: "/".to_string(),
            service: None,
            selected_upstream: None,
            upstream_path: None,

            // Protocol flags that help figure out what to do with the request.
            is_upgrade_req: false,
            ws_opened: false,
            protocol_version: None,

            // Required for gRPC.
            upstream_authority: None,

            // Traffic/Circuit-breaker.
            cb_started: false,
            upstream_outcome: None,

            // Peer info - filled out during hydration
            peer_ip: Ipv4Addr::UNSPECIFIED.into(),

            // Device related data.
            extensions: Extensions::new(),

            // Request normalization
            normalized_request: None,
            normalized: false,
        }
    }

    pub fn hydrate_from_session(&mut self, session: &Session) {
        debug_assert!(!self.hydrated, "RequestCtx hydrated twice");
        if self.hydrated {
            return;
        }

        let req = session.req_header();

        self.method = Some(req.method.clone());
        self.original_uri = Some(req.uri.clone());
        self.query_string = req
            .uri
            .query()
            .map(ToOwned::to_owned)
            .filter(|s| !s.is_empty());
        self.headers = req.headers.clone();
        self.route_path = req.uri.path().to_string();
        self.is_upgrade_req = session.is_upgrade_req();
        self.protocol_version = Some(req.version);
        self.peer_ip = match session.client_addr() {
            Some(PingoraSocketAddr::Inet(addr)) => addr.ip(),
            _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        };

        self.extensions.insert(RequestId::default());

        self.hydrated = true;
        self.normalized = false;
    }

    pub fn normalize_request(&mut self) -> Result<(), RequestRejectError> {
        debug_assert!(self.hydrated, "normalize before hydrate");
        debug_assert!(!self.normalized, "normalize called twice");

        if self.is_upgrade_req {
            self.normalize_ws_handshake()?;
        } else {
            self.normalize_http_request()?;
        }

        Ok(())
    }

    fn normalize_ws_handshake(&mut self) -> Result<(), RequestRejectError> {
        // Method must be GET
        if self.method != Some(Method::GET) {
            return Err(RequestRejectError::InvalidMethod);
        }

        // Normalize path (security + routing)
        let path = match normalize_path(&self.route_path) {
            NormalizationOutcome::Accept(p) | NormalizationOutcome::Rewrite { value: p, .. } => p,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidPath);
            }
        };

        self.route_path = path.as_str().to_string();

        // Header *validation only*
        for (name, value) in self.headers.iter() {
            name.as_str(); // validate name
            value
                .to_str()
                .map_err(|_| RequestRejectError::InvalidHeaders)?;

            if value.as_bytes().contains(&0) {
                return Err(RequestRejectError::InvalidHeaders);
            }
        }

        self.normalized = true;
        Ok(())
    }

    fn normalize_http_request(&mut self) -> Result<(), RequestRejectError> {
        debug_assert!(self.hydrated, "normalize before hydrate");
        debug_assert!(!self.normalized, "normalize called twice");

        if self.normalized {
            return Ok(());
        }

        let raw_path = self.route_path.as_str();

        let normalized_path = match normalize_path(raw_path) {
            NormalizationOutcome::Accept(p) => p,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidPath);
            }
        };

        let raw_query = self.query_string.as_deref().unwrap_or("");

        let canonical_query = match normalize_query(raw_query) {
            NormalizationOutcome::Accept(q) => q,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidQueryString);
            }
        };

        // Header normalization is protocol-specific.
        let protocol_normalization_mode = match self.protocol_version {
            Some(Version::HTTP_2) => ProtocolNormalizationMode::Http2,
            _ => ProtocolNormalizationMode::Http1,
        };

        let normalized_headers =
            match normalize_headers(&self.headers, &protocol_normalization_mode) {
                NormalizationOutcome::Accept(h) => h,
                NormalizationOutcome::Rewrite { value, .. } => value,
                NormalizationOutcome::Reject { .. } => {
                    return Err(RequestRejectError::InvalidHeaders);
                }
            };

        let method = self
            .method
            .as_ref()
            .ok_or(RequestRejectError::MissingMethod)?;

        self.normalized_request = Some(NormalizedRequest::new(
            method.clone(),
            normalized_path,
            canonical_query,
            normalized_headers,
        ));

        self.normalized = true;
        Ok(())
    }

    /// Path used when proxying upstream
    pub fn upstream_path(&self) -> &str {
        self.upstream_path
            .as_deref()
            .unwrap_or(self.route_path.as_str())
    }

    /// Returns the upstream authority (host:port) to use for HTTP/2 requests.
    ///
    /// This is typically set when proxying to HTTP/2 backends that require
    /// a specific :authority pseudo-header value.
    pub fn upstream_authority(&self) -> Option<&str> {
        self.upstream_authority.as_deref()
    }

    pub fn is_http2(&self) -> bool {
        self.protocol_version == Some(Version::HTTP_2)
    }

    pub fn original_uri_str(&self) -> Option<String> {
        self.original_uri.as_ref().map(|u| u.to_string())
    }

    pub fn method_str(&self) -> &str {
        debug_assert!(self.normalized_request.is_some());
        self.normalized_request
            .as_ref()
            .expect("request not normalized. this is a bug.")
            .method()
            .as_str()
    }

    pub fn method(&self) -> &Method {
        debug_assert!(self.normalized_request.is_some());
        self.normalized_request
            .as_ref()
            .expect("request not normalized. this is a bug.")
            .method()
    }

    /// Internal canonical representation of the request path.
    pub fn canonical_path(&self) -> &str {
        self.normalized_request
            .as_ref()
            .expect("request not normalized. this is a bug.")
            .path()
            .as_str()
    }

    pub fn request_id(&self) -> Option<String> {
        self.extensions.get::<RequestId>().map(|id| id.0.clone())
    }
}
