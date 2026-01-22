use crate::ctx::{RequestCtx, RequestId, RequestRejectError};
use http::{HeaderMap, HeaderValue, Method, Uri, Version};

//-----------------------------------------------------------------------------
// Test helpers
//-----------------------------------------------------------------------------

/// Build a RequestCtx that is "logically hydrated" without needing a real pingora Session.
///
/// This works because tests are in the same module as RequestCtx and can access private fields.
fn hydrated_ctx_base() -> RequestCtx {
    let mut ctx = RequestCtx::empty();

    // Mimic what hydrate_from_session() would have set.
    ctx.hydrated = true;
    ctx.normalized = false;

    // Provide sane defaults for normalization.
    ctx.method = Some(Method::GET);
    ctx.route_path = "/".to_string();
    ctx.query_string = None;
    ctx.headers = HeaderMap::new();
    ctx.protocol_version = Some(Version::HTTP_11);
    ctx.is_upgrade_req = false;

    // Set the original URI for helper-method tests.
    ctx.original_uri = Some(Uri::from_static("http://example.test/"));

    // Mimic RequestId insertion.
    ctx.extensions.insert(RequestId::default());

    ctx
}

//-----------------------------------------------------------------------------
// Websocket handshake normalization
//-----------------------------------------------------------------------------
#[test]
fn ws_handshake_rejects_non_get_method() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = true;
    ctx.method = Some(Method::POST); // WS handshake must be GET
    ctx.route_path = "/ws".to_string();

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(matches!(result, Err(RequestRejectError::InvalidMethod)));
    assert!(!ctx.normalized, "should not mark normalized on rejection");
}

#[test]
fn ws_handshake_rejects_invalid_path() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = true;
    ctx.method = Some(Method::GET);

    // Choose a path that *should* be rejected by normalize_path() for security reasons.
    ctx.route_path = "/../secret".to_string();

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(matches!(result, Err(RequestRejectError::InvalidPath)));
    assert!(!ctx.normalized, "should not mark normalized on rejection");
}

#[test]
fn ws_handshake_rejects_non_utf8_header_value() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = true;
    ctx.method = Some(Method::GET);
    ctx.route_path = "/ws".to_string();

    // HeaderValue can contain non-UTF8 bytes; to_str() will fail and validation rejects.
    let non_utf8 =
        HeaderValue::from_bytes(b"\xFF\xFE").expect("HeaderValue should allow non-UTF8 bytes");
    ctx.headers.insert("x-test", non_utf8);

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(matches!(result, Err(RequestRejectError::InvalidHeaders)));
    assert!(!ctx.normalized);
}

#[test]
fn ws_handshake_accepts_and_marks_normalized() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = true;
    ctx.method = Some(Method::GET);
    ctx.route_path = "/ws".to_string();
    ctx.headers
        .insert("host", HeaderValue::from_static("example.test"));
    ctx.headers
        .insert("upgrade", HeaderValue::from_static("websocket"));

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(result.is_ok());
    assert!(
        ctx.normalized,
        "WS handshake should mark ctx.normalized = true"
    );
    // WS path normalization updates route_path (even if it is a no-op).
    assert_eq!(ctx.route_path, "/ws");
}

//-----------------------------------------------------------------------------
// HTTP request normalization
//-----------------------------------------------------------------------------
#[test]
fn http_normalize_builds_normalized_request_and_marks_normalized() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = false;
    ctx.method = Some(Method::GET);
    ctx.route_path = "/books".to_string();
    ctx.query_string = Some("b=2&a=1".to_string()); // we won't assume canonical ordering here
    ctx.headers
        .insert("host", HeaderValue::from_static("example.test"));
    ctx.protocol_version = Some(Version::HTTP_11);

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(result.is_ok());
    assert!(
        ctx.normalized,
        "HTTP request should mark ctx.normalized = true"
    );

    let nr = ctx
        .normalized_request
        .as_ref()
        .expect("normalized_request missing");
    assert_eq!(nr.method(), &Method::GET);

    // For a simple, already-normal path, canonical path should match.
    assert_eq!(ctx.canonical_path(), "/books");
    assert_eq!(nr.path().as_str(), "/books");
}

#[test]
fn http_normalize_uses_http2_mode_when_version_is_http2() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.is_upgrade_req = false;
    ctx.method = Some(Method::GET);
    ctx.route_path = "/grpc.Service/Method".to_string();
    ctx.query_string = None;
    ctx.protocol_version = Some(Version::HTTP_2);
    ctx.headers
        .insert("host", HeaderValue::from_static("example.test"));

    // Act
    let result = ctx.normalize_request();

    // Assert
    assert!(result.is_ok());
    assert!(ctx.is_http2());
    assert!(ctx.normalized_request.is_some());
}

//-----------------------------------------------------------------------------
// Small utility methods
//-----------------------------------------------------------------------------
#[test]
fn upstream_path_returns_override_when_set_otherwise_route_path() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.route_path = "/from-route".to_string();

    // Act and Assert (no override)
    assert_eq!(ctx.upstream_path(), "/from-route");

    // Arrange (override)
    ctx.upstream_path = Some("/override".to_string());

    // Act and Assert (override)
    assert_eq!(ctx.upstream_path(), "/override");
}

#[test]
fn upstream_authority_getter() {
    // Arrange
    let mut ctx = hydrated_ctx_base();

    // Act and Assert (none)
    assert_eq!(ctx.upstream_authority(), None);

    // Arrange (set)
    ctx.upstream_authority = Some("backend.internal:8443".to_string());

    // Act and Assert
    assert_eq!(ctx.upstream_authority(), Some("backend.internal:8443"));
}

#[test]
fn method_and_original_uri_helpers() {
    // Arrange
    let mut ctx = hydrated_ctx_base();
    ctx.method = Some(Method::PUT);
    ctx.original_uri = Some(Uri::from_static("http://example.test/hello?x=1"));

    // Act
    let method_str = ctx.method_str();
    let uri_str = ctx.original_uri_str();

    // Assert
    assert_eq!(method_str, Some("PUT"));
    assert_eq!(uri_str.as_deref(), Some("http://example.test/hello?x=1"));
}

#[test]
#[should_panic(expected = "request not normalized")]
fn canonical_path_panics_if_not_normalized() {
    // Arrange
    let ctx = hydrated_ctx_base();

    // Act and Assert
    let _ = ctx.canonical_path();
}
