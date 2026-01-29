use crate::ctx::RequestId;
use crate::ctx::request::error::RequestRejectError;
use crate::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode, normalize_headers, normalize_path,
    normalize_query,
};
use crate::ctx::request::{NormalizedHeaders, NormalizedRequest};
use crate::route::types::RouteId;
use crate::runtime::UpstreamId;
use crate::traffic_management::{AdmissionGuard, ServiceId, UpstreamOutcome};
use crate::ws_connection_management::WsConnectionGuard;
use http::{Extensions, HeaderMap, HeaderName, HeaderValue, Method, Uri, Version};
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

    /// Optional override for the upstream request path
    pub upstream_path: Option<String>,

    /// Remote IP of the TCP connection (authoritative)
    pub peer_ip: IpAddr,

    /// Was a websocket connection opened?
    pub ws_opened: bool,

    /// Upstream authority for HTTP/2 requests.
    pub upstream_authority: Option<String>,

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default).
    pub extensions: Extensions,

    /// Normalized request representation for routing and processing.
    pub normalized_request: NormalizedRequest,

    /// Route ID for routing decisions.
    pub route_id: Option<RouteId>,

    /// Selected upstream and outcome
    pub selected_upstream: Option<(ServiceId, UpstreamId)>,

    /// Outcome of upstream selection
    pub upstream_outcome: Option<UpstreamOutcome>,

    /// Circuit breaker started?
    pub cb_started: bool,
}

impl Default for RequestCtx {
    fn default() -> Self {
        Self::empty()
    }
}

/// Hydration API
impl RequestCtx {
    pub fn empty() -> Self {
        Self {
            route_id: None,

            // Request lifecycle-related.
            hydrated: false,
            admission_guard: None,
            ws_guard: None,

            // Upstream/routing related.
            service: None,
            selected_upstream: None,
            upstream_path: None,

            // Protocol flag(s) that help figure out what to do with the request.
            ws_opened: false,

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
            normalized_request: NormalizedRequest::default(),
        }
    }

    /// Create a boundary to decouple session from logic.
    /// This makes testing the hydration/normalization code easier.
    pub fn hydrate_from_session(&mut self, session: &Session) -> Result<(), RequestRejectError> {
        let request_header = session.req_header();
        let is_upgrade_req = session.is_upgrade_req();
        // Get the client IP from Pingora.
        let peer_ip = match session.client_addr() {
            Some(PingoraSocketAddr::Inet(addr)) => addr.ip(),
            _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        };

        self.hydrate(
            &request_header.uri,
            &request_header.method,
            &request_header.headers,
            &request_header.version,
            is_upgrade_req,
            peer_ip,
        )?;

        Ok(())
    }

    pub(crate) fn hydrate(
        &mut self,
        uri: &Uri,
        method: &Method,
        headers: &HeaderMap,
        protocol_version: &Version,
        is_upgrade_req: bool,
        peer_ip: IpAddr,
    ) -> Result<(), RequestRejectError> {
        debug_assert!(!self.hydrated, "Already hydrated, cannot hydrate again");
        // Generate a new request ID.
        self.extensions.insert(RequestId::default());

        // Set the client IP.
        self.peer_ip = self.peer_ip.max(peer_ip);

        // Do header normalization early as it may produce a protocol-related violation.
        // This will short-circuit the request if it's invalid while preventing unused allocations.
        let normalized_headers = if is_upgrade_req {
            self.normalize_ws_handshake(method, headers)?
        } else {
            self.normalize_http_request(protocol_version, headers)?
        };

        // Normalize the path.
        let normalized_path = match normalize_path(uri.path()) {
            NormalizationOutcome::Accept(p) => p,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidPath);
            }
        };

        // Normalize the query string.
        let raw_query = uri.query().unwrap_or_default();
        let canonical_query = match normalize_query(raw_query) {
            NormalizationOutcome::Accept(q) => q,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidQueryString);
            }
        };

        self.normalized_request = NormalizedRequest::new(
            uri.clone(),
            method.clone(),
            normalized_path,
            canonical_query,
            normalized_headers,
            *protocol_version,
            is_upgrade_req,
        );

        self.hydrated = true;
        Ok(())
    }

    pub(crate) fn normalize_ws_handshake(
        &self,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<NormalizedHeaders, RequestRejectError> {
        // Method must be GET for a WS handshake.
        if method != Method::GET {
            return Err(RequestRejectError::InvalidMethod);
        }

        // Header validation ONLY.
        // Mutating the headers here would cause the handshake to fail.
        for (name, value) in headers.iter() {
            name.as_str(); // validate name
            value
                .to_str()
                .map_err(|_| RequestRejectError::InvalidHeaders)?;

            if value.as_bytes().contains(&0) {
                return Err(RequestRejectError::InvalidHeaders);
            }
        }
        let normalized_headers = NormalizedHeaders::new(headers.clone());

        Ok(normalized_headers)
    }

    pub(crate) fn normalize_http_request(
        &self,
        protocol_version: &Version,
        headers: &HeaderMap,
    ) -> Result<NormalizedHeaders, RequestRejectError> {
        // Header normalization is protocol-specific, meaning that
        // the protocol ultimately decides which set of rules to apply to the headers in the
        // normalize_headers() function.
        let protocol_normalization_mode = match *protocol_version {
            Version::HTTP_2 => ProtocolNormalizationMode::Http2,
            _ => ProtocolNormalizationMode::Http1,
        };

        let normalized_headers = match normalize_headers(headers, &protocol_normalization_mode) {
            NormalizationOutcome::Accept(h) => h,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidHeaders);
            }
        };

        Ok(normalized_headers)
    }
}

/// HTTP/2 API
impl RequestCtx {
    /// Returns the upstream authority (host:port) to use for HTTP/2 requests.
    ///
    /// This is typically set when proxying to HTTP/2 backends that require
    /// a specific :authority pseudo-header value.
    pub fn upstream_authority(&self) -> Option<&str> {
        self.upstream_authority.as_deref()
    }

    pub fn is_http2(&self) -> bool {
        debug_assert!(self.hydrated);
        self.normalized_request.is_http2()
    }
}

/// Websocket API
impl RequestCtx {
    pub fn is_upgrade_req(&self) -> bool {
        debug_assert!(self.hydrated);
        self.normalized_request.is_upgrade_req()
    }
}

/// Request Header API
impl RequestCtx {
    pub fn headers(&self) -> &HeaderMap {
        debug_assert!(self.hydrated);
        self.normalized_request.headers()
    }

    pub(crate) fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        debug_assert!(self.hydrated);
        self.normalized_request.insert_header(name, value);
    }

    pub(crate) fn remove_header(&mut self, name: &str) {
        debug_assert!(self.hydrated);
        self.normalized_request.remove_header(name);
    }
}

/// Request Path API
impl RequestCtx {
    /// Path used when proxying upstream
    pub fn upstream_path(&self) -> &str {
        self.upstream_path
            .as_deref()
            .unwrap_or(self.canonical_path())
    }

    /// Will return the full original URI as received the proxy.
    /// This may include the scheme, host, and port.
    /// Or, just the path with an optional query string.
    pub fn original_uri_string(&self) -> String {
        debug_assert!(self.hydrated);
        self.normalized_request.original_uri().to_string()
    }

    /// Will return the original URI path.
    /// This is the path as it was received by the proxy.
    /// This may include the path with an optional query string.
    /// e.g., /foo/bar or /foo/bar?a=b
    pub fn original_uri_path(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.original_uri().path()
    }

    /// Internal canonical representation of the request path.
    pub fn canonical_path(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.path().as_str()
    }

    pub(crate) fn set_canonical_path(&mut self, path: String) {
        debug_assert!(self.hydrated);
        self.normalized_request.set_path(path);
    }
}

/// Method API
impl RequestCtx {
    pub fn method_str(&self) -> &str {
        self.method().as_str()
    }

    pub fn method(&self) -> &Method {
        debug_assert!(self.hydrated);
        self.normalized_request.method()
    }

    /// Return true if the method is allowed to have a body.
    pub fn has_defined_body_semantics(&self) -> bool {
        let method = self.method();
        method == Method::POST || method == Method::PATCH || method == Method::PUT
    }

    /// Return true for the special case of CONNECT method
    /// Conceptually, the presence of a body does not matter for a CONNECT request.
    /// HTTP semantics are discarded after the CONNECT request is established.
    /// After that data is actually transferred.
    pub fn body_presence_is_irrelevant(&self) -> bool {
        let method = self.method();
        method == Method::CONNECT
    }
}

/// Request ID API
impl RequestCtx {
    pub fn request_id(&self) -> Option<String> {
        self.extensions.get::<RequestId>().map(|id| id.0.clone())
    }
}
