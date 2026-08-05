use crate::execution::DownstreamSni;
use crate::execution::ctx::RequestId;
use crate::execution::ctx::request::error::RequestRejectError;
use crate::execution::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode, normalize_headers, normalize_path,
    normalize_query,
};
use crate::execution::ctx::request::{
    NormalizedHeaders, NormalizedRequest, NormalizedRequestParams, RequestSource,
};
use crate::execution::enrichment::user_agent::ClientIdentity;
use crate::execution::route::types::RouteId;
use crate::execution::traffic::{AdmissionGuard, ServiceId, UpstreamOutcome};
use crate::execution::ws_connection_management::WsConnectionGuard;
use crate::runtime::UpstreamId;
use http::header::HOST;
use http::{Extensions, HeaderMap, Method, Version, uri::Authority};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use tracing::Span;

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
    pub(crate) upstream_path: Option<String>,

    /// Remote IP of the TCP connection (authoritative)
    pub peer_ip: IpAddr,

    /// Was a websocket connection opened?
    pub ws_opened: bool,

    /// Upstream authority for HTTP/2 requests.
    pub upstream_authority: Option<String>,

    /// Request-scoped typed extensions (NOT forwarded, NOT logged by default).
    pub extensions: Extensions,

    /// Normalized request representation for routing and processing.
    normalized_request: NormalizedRequest,

    /// Route ID for routing decisions.
    pub route_id: Option<RouteId>,

    /// Selected upstream and outcome
    pub selected_upstream: Option<(ServiceId, UpstreamId)>,

    /// Outcome of upstream selection
    pub upstream_outcome: Option<UpstreamOutcome>,

    /// Circuit breaker started?
    pub cb_started: bool,

    /// Root tracing request span.
    pub request_span: Option<Span>,

    /// Request start time for latency measurement.
    pub request_start: std::time::Instant,
}

impl Default for RequestCtx {
    fn default() -> Self {
        Self::empty()
    }
}

/// Hydration API
#[hotpath::measure_all]
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

            // Observability.
            request_span: None,
            request_start: std::time::Instant::now(),
        }
    }

    /// Create a boundary to decouple session from logic.
    /// This makes testing the hydration/normalization code easier.
    pub fn hydrate_from_session<S: RequestSource>(
        &mut self,
        src: &S,
    ) -> Result<(), RequestRejectError> {
        debug_assert!(!self.hydrated, "Already hydrated, cannot hydrate again");

        // Generate a new request ID.
        self.extensions.insert(RequestId::default());

        // Set the client IP.
        if self.peer_ip.is_unspecified() {
            self.peer_ip = src.net_peer_ip();
        }

        //---------------------------------------------------------------------
        // Header normalization.
        //---------------------------------------------------------------------
        // Do header normalization early as it may produce a protocol-related violation.
        // This will short-circuit the request if it's invalid while preventing unused allocations.
        let normalized_headers = if src.http_is_upgrade_req() {
            self.normalize_ws_handshake(src.http_method(), src.http_headers())?
        } else {
            self.normalize_http_request(&src.http_version(), src.http_headers())?
        };

        //---------------------------------------------------------------------
        // Extract canonical authority (H2 first, H1 fallback)
        //---------------------------------------------------------------------
        let authority_str = if let Some(auth) = src.http_uri().authority() {
            auth.as_str()
        } else if let Some(host_header) = normalized_headers.as_map().get(HOST) {
            host_header
                .to_str()
                .map_err(|_| RequestRejectError::InvalidHostHeader)?
        } else {
            return Err(RequestRejectError::InvalidHostHeader);
        };

        // Strip trailing dot (RFC 3986 allowance)
        let authority_str = authority_str.trim_end_matches('.');

        if authority_str.is_empty() {
            return Err(RequestRejectError::InvalidHostHeader);
        }

        // Parse safely (handles host:port and IPv6 correctly)
        let authority = Authority::from_str(authority_str)
            .map_err(|_| RequestRejectError::InvalidHostHeader)?;

        let host = authority.host().to_ascii_lowercase();

        //---------------------------------------------------------------------
        // SNI
        //---------------------------------------------------------------------
        // Extract SNI, if present, from the SSL digest.
        let mut sni_host: Option<String> = None;
        if let Some(digest) = src.net_digest()
            && let Some(ssl_digest) = &digest.ssl_digest
        {
            let maybe_sni = &ssl_digest.extension.get::<DownstreamSni>();
            if let Some(sni) = maybe_sni {
                sni_host = Some(sni.to_ascii_lowercase());
            }
        }

        // Enforce SNI/Host matching - they must match if SNI is present.
        if let Some(sni) = sni_host.clone()
            && sni.as_str() != host
        {
            return Err(RequestRejectError::HostSniMismatch);
        }

        //---------------------------------------------------------------------
        // Normalize the path.
        //---------------------------------------------------------------------
        let normalized_path = match normalize_path(src.http_uri().path()) {
            NormalizationOutcome::Accept(p) => p,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidPath);
            }
        };

        //---------------------------------------------------------------------
        // Normalize the query string.
        //---------------------------------------------------------------------
        let raw_query = src.http_uri().query().unwrap_or_default();
        let canonical_query = match normalize_query(raw_query) {
            NormalizationOutcome::Accept(q) => q,
            NormalizationOutcome::Rewrite { value, .. } => value,
            NormalizationOutcome::Reject { .. } => {
                return Err(RequestRejectError::InvalidQueryString);
            }
        };

        self.normalized_request = NormalizedRequestParams {
            host,
            sni_host,
            original_uri: src.http_uri().clone(),
            method: src.http_method().clone(),
            path: normalized_path,
            query: canonical_query,
            headers: normalized_headers,
            protocol_version: src.http_version(),
            is_upgrade_req: src.http_is_upgrade_req(),
        }
        .into();

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

    /// Normally this function would not be used outside a unit test or a CLI command
    /// that makes a synthetic request.
    pub fn set_normalized_request(&mut self, request: NormalizedRequest) {
        self.normalized_request = request;
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

    /// Returns the authority (host:port) of the downstream request URI.
    ///
    /// Present for HTTP/2 requests, where it carries the `:authority`
    /// pseudo-header value; HTTP/1.1 requests in origin-form have no URI
    /// authority and return `None` (their authority lives in the `Host`
    /// header instead).
    pub fn downstream_authority(&self) -> Option<&str> {
        debug_assert!(self.hydrated);
        self.normalized_request
            .original_uri()
            .authority()
            .map(|a| a.as_str())
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
}

/// WASM Device API
///
use http::{HeaderName, HeaderValue};

impl RequestCtx {
    pub(crate) fn set_canonical_path(&mut self, path: String) {
        debug_assert!(self.hydrated);
        self.normalized_request.set_path(path);
    }

    pub(crate) fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        debug_assert!(self.hydrated);
        self.normalized_request.insert_header(name, value);
    }

    pub(crate) fn append_header(&mut self, name: HeaderName, value: HeaderValue) {
        debug_assert!(self.hydrated);
        self.normalized_request.append_header(name, value);
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

    /// URI used when proxying upstream.
    /// Appends the original query string to the upstream path when the
    /// request carried one.
    pub fn upstream_uri(&self) -> String {
        let query = self.query_string();
        if query.is_empty() {
            self.upstream_path().to_string()
        } else {
            format!("{}?{}", self.upstream_path(), query)
        }
    }

    /// Will return the full original URI as received the proxy.
    /// This may include the scheme, host, and port.
    /// Or, just the path with an optional query string.
    pub(crate) fn original_uri_string(&self) -> String {
        debug_assert!(self.hydrated);
        self.normalized_request.original_uri().to_string()
    }

    /// Will return the original URI path.
    /// This is the path as it was received by the proxy.
    /// This may include the path with an optional query string, e.g., /foo/bar or /foo/bar?a=b
    pub(crate) fn original_uri_path(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.original_uri().path()
    }

    /// Internal canonical representation of the request path.
    pub fn canonical_path(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.path().as_str()
    }

    /// The SNI if present, otherwise HOST header value.
    pub fn effective_host(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.effective_host()
    }

    pub(crate) fn query_string(&self) -> &str {
        debug_assert!(self.hydrated);
        self.normalized_request.query().raw()
    }

    pub(crate) fn scheme(&self) -> &str {
        if self.normalized_request.sni_host.is_some() {
            "https"
        } else {
            "http"
        }
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
    pub(crate) fn has_defined_body_semantics(&self) -> bool {
        let method = self.method();
        method == Method::POST || method == Method::PATCH || method == Method::PUT
    }
}

/// Request Extensions API
impl RequestCtx {
    pub fn request_id(&self) -> Option<String> {
        self.extensions.get::<RequestId>().map(|id| id.0.clone())
    }

    pub(crate) fn identity(&self) -> Option<&ClientIdentity> {
        self.extensions.get::<ClientIdentity>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ctx::request::RequestSource;
    use crate::execution::ctx::request::error::RequestRejectError;
    use http::header::HOST;
    use http::{HeaderMap, HeaderValue, Method, Uri, Version};
    use pingora::prelude::Session;
    use pingora::protocols::Digest;
    use pretty_assertions::assert_eq;
    use std::net::IpAddr;
    use tokio::io::{AsyncWriteExt, duplex};

    //-----------------------------------------------------------------------------
    // Test helpers
    //-----------------------------------------------------------------------------
    struct FakeRequest {
        uri: Uri,
        method: Method,
        headers: HeaderMap,
        version: Version,
        upgrade: bool,
        peer_ip: IpAddr,
        digest: Option<Digest>,
    }

    impl RequestSource for FakeRequest {
        fn http_uri(&self) -> &Uri {
            &self.uri
        }
        fn http_method(&self) -> &Method {
            &self.method
        }
        fn http_headers(&self) -> &HeaderMap {
            &self.headers
        }
        fn http_version(&self) -> Version {
            self.version
        }
        fn http_is_upgrade_req(&self) -> bool {
            self.upgrade
        }

        fn net_peer_ip(&self) -> IpAddr {
            self.peer_ip
        }

        fn net_digest(&self) -> Option<&Digest> {
            self.digest.as_ref()
        }
    }

    pub(crate) struct RawHttpRequest {
        method: String,
        target: String,
        version: &'static str,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
        body: Vec<u8>,
    }

    impl RawHttpRequest {
        pub(crate) fn new(method: impl Into<String>, target: impl Into<String>) -> Self {
            Self {
                method: method.into(),
                target: target.into(),
                version: "HTTP/1.1",
                headers: Vec::new(),
                body: Vec::new(),
            }
        }

        pub(crate) fn header(mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> Self {
            self.headers.push((
                k.as_ref().as_bytes().to_vec(),
                v.as_ref().as_bytes().to_vec(),
            ));
            self
        }

        pub(crate) fn header_bytes(mut self, k: impl AsRef<[u8]>, v: impl AsRef<[u8]>) -> Self {
            self.headers
                .push((k.as_ref().to_vec(), v.as_ref().to_vec()));
            self
        }

        pub(crate) fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
            self.body = body.into();
            self
        }

        pub(crate) fn build(self) -> Vec<u8> {
            let mut out = Vec::new();

            // request line
            out.extend_from_slice(
                format!("{} {} {}\r\n", self.method, self.target, self.version).as_bytes(),
            );

            // headers
            for (k, v) in self.headers {
                out.extend_from_slice(&k);
                out.extend_from_slice(b": ");
                out.extend_from_slice(&v);
                out.extend_from_slice(b"\r\n");
            }

            // header/body separator
            out.extend_from_slice(b"\r\n");

            // body
            out.extend_from_slice(&self.body);

            out
        }
    }

    async fn make_h1_session(request: &[u8]) -> Session {
        // duplex() creates a pair of in-memory streams that act like two sockets.
        let (mut client_side, server_side) = duplex(64 * 1024);
        // Build a real Session backed by memory IO.
        let mut session = Session::new_h1(Box::new(server_side));
        // Send synthetic HTTP request.
        client_side.write_all(request).await.unwrap();
        // Let pingora parse request.
        assert!(session.read_request().await.unwrap());
        session
    }

    //-----------------------------------------------------------------------------
    // Websocket handshake normalization
    //-----------------------------------------------------------------------------
    #[tokio::test]
    async fn hydrate_from_session_basic() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/foo")
            .header("Host", "example.com")
            .header("Content-Type", "application/json")
            .body(r#"{"a":1}"#)
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        ctx.hydrate_from_session(&session).unwrap();

        // Assert
        assert_eq!(ctx.method(), "GET");
        assert_eq!(ctx.canonical_path(), "/foo");
    }

    #[tokio::test]
    async fn ws_handshake_rejects_non_get_method() {
        // Arrange
        let request = RawHttpRequest::new("POST", "/ws")
            .header("Host", "example.test")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        let result = ctx.hydrate_from_session(&session);

        // Assert
        assert!(matches!(result, Err(RequestRejectError::InvalidMethod)));
        assert!(!ctx.hydrated, "should not mark hydrated on rejection");
    }

    #[tokio::test]
    async fn ws_handshake_rejects_invalid_path() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/../secret")
            .header("Host", "example.test")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        let result = ctx.hydrate_from_session(&session);

        // Assert
        assert!(matches!(result, Err(RequestRejectError::InvalidPath)));
        assert!(!ctx.hydrated, "should not mark hydrated on rejection");
    }

    #[tokio::test]
    async fn ws_handshake_rejects_non_utf8_header_value() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/ws")
            .header("Host", "example.test")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header_bytes("X-Test", b"\xFF\xFE")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        let result = ctx.hydrate_from_session(&session);

        // Assert
        assert!(matches!(result, Err(RequestRejectError::InvalidHeaders)));
        assert!(!ctx.hydrated);
    }

    #[tokio::test]
    async fn ws_handshake_accepts_and_marks_normalized() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/ws")
            .header("Host", "example.test")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        let result = ctx.hydrate_from_session(&session);

        // Assert
        assert!(result.is_ok());
        assert!(ctx.hydrated, "WS handshake should mark ctx.hydrated = true");
        assert_eq!(ctx.canonical_path(), "/ws"); // WS path normalization updates route_path (even if it is a no-op).
    }

    //-----------------------------------------------------------------------------
    // HTTP request normalization
    //-----------------------------------------------------------------------------
    #[tokio::test]
    async fn http_normalize_builds_normalized_request_and_marks_normalized() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/books?b=2&a=1")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();

        // Act
        let result = ctx.hydrate_from_session(&session);

        // Assert
        assert!(result.is_ok());
        assert!(ctx.hydrated, "HTTP request should mark ctx.hydrated = true");
        assert_eq!(ctx.method(), &Method::GET);
        assert_eq!(ctx.canonical_path(), "/books");
        assert_eq!(ctx.original_uri_path(), "/books");
    }

    #[test]
    fn hydrate_runs_http2_normalization() {
        let mut headers = HeaderMap::new();
        headers.append(HOST, HeaderValue::from_static("example.test"));

        // intentionally needs rewrite (OWS trim + duplicate folding)
        headers.append("x-test", HeaderValue::from_static(" a "));
        headers.append("x-test", HeaderValue::from_static("b"));

        let mut ctx = RequestCtx::empty();

        let req = FakeRequest {
            uri: Uri::from_static("https://example.test/grpc.Service/Method"),
            method: Method::GET,
            headers,
            version: Version::HTTP_2,
            upgrade: false,
            peer_ip: "127.0.0.1".parse().unwrap(),
            digest: None,
        };
        let _ = ctx.hydrate_from_session(&req);

        // Assert
        assert!(ctx.hydrated);
        assert!(ctx.is_http2());
        assert_eq!(ctx.headers().get("x-test").unwrap(), "a, b");
    }

    //-----------------------------------------------------------------------------
    // Small utility methods
    //-----------------------------------------------------------------------------
    #[tokio::test]
    async fn upstream_path_returns_override_when_set() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/from-route")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);
        ctx.upstream_path = Some("/override".to_string());

        // Act
        let result = ctx.upstream_path();

        // Assert
        assert_eq!(result, "/override");
    }

    #[tokio::test]
    async fn upstream_path_returns_canonical_path_when_not_set() {
        // Arrange
        let expected_path = "/from-route";
        let request = RawHttpRequest::new("GET", expected_path)
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.upstream_path();

        // Assert
        assert_eq!(result, expected_path);
    }

    #[tokio::test]
    async fn upstream_uri_includes_query_string() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/api?action=search&q=hello+world&page=1")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.upstream_uri();

        // Assert
        assert_eq!(result, "/api?action=search&q=hello+world&page=1");
    }

    #[tokio::test]
    async fn upstream_uri_omits_query_delimiter_when_no_query() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/api")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.upstream_uri();

        // Assert
        assert_eq!(result, "/api");
    }

    #[tokio::test]
    async fn upstream_uri_appends_query_to_upstream_path_override() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/from-route?a=1&b=2")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);
        ctx.upstream_path = Some("/override".to_string());

        // Act
        let result = ctx.upstream_uri();

        // Assert
        assert_eq!(result, "/override?a=1&b=2");
    }

    #[tokio::test]
    async fn upstream_authority_return_none_when_not_set() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/books?b=2&a=1")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.upstream_authority();

        // Assert
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn upstream_authority_getter_should_return_authority_when_set() {
        // Arrange
        let request = RawHttpRequest::new("GET", "/books?b=2&a=1")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);
        let expected_authority = "backend.internal:8443";
        ctx.upstream_authority = Some(expected_authority.to_string());

        // Act
        let result = ctx.upstream_authority();

        // Assert
        assert_eq!(result, Some(expected_authority));
    }

    #[tokio::test]
    async fn method_str_is_normalized_if_set() {
        // Arrange
        let expected_str = "PUT";
        let request = RawHttpRequest::new(expected_str, "/books?b=2&a=1")
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Arrange
        let expected_str = "PUT";

        // Act
        let method_str = ctx.method_str();

        // Assert
        assert_eq!(method_str, expected_str);
    }

    #[tokio::test]
    async fn original_uri_is_intact() {
        // Arrange
        let expected_uri = "/hello?x=1";
        let request = RawHttpRequest::new("GET", expected_uri)
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.original_uri_string();

        // Assert
        assert_eq!(result, expected_uri);
    }

    #[tokio::test]
    async fn original_uri_path_is_intact() {
        // Arrange
        let expected_path = "/hello";
        let full_path = format!("{}?x=1", expected_path);
        let request = RawHttpRequest::new("GET", full_path)
            .header("Host", "example.test")
            .build();
        let session = make_h1_session(&request).await;
        let mut ctx = RequestCtx::empty();
        let _ = ctx.hydrate_from_session(&session);

        // Act
        let result = ctx.original_uri_path();

        // Assert
        assert_eq!(result, expected_path);
    }
}
