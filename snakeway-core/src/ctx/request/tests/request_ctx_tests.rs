use crate::ctx::{RequestCtx, RequestRejectError};
use http::Method;
use pingora::prelude::Session;
use pretty_assertions::assert_eq;
use tokio::io::{AsyncWriteExt, duplex};
//-----------------------------------------------------------------------------
// Test helpers
//-----------------------------------------------------------------------------
pub struct RawHttpRequest {
    method: String,
    target: String,
    version: &'static str,
    headers: Vec<(Vec<u8>, Vec<u8>)>,
    body: Vec<u8>,
}

impl RawHttpRequest {
    pub fn new(method: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            target: target.into(),
            version: "HTTP/1.1",
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }

    pub fn header(mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> Self {
        self.headers.push((
            k.as_ref().as_bytes().to_vec(),
            v.as_ref().as_bytes().to_vec(),
        ));
        self
    }

    pub fn header_bytes(mut self, k: impl AsRef<[u8]>, v: impl AsRef<[u8]>) -> Self {
        self.headers
            .push((k.as_ref().to_vec(), v.as_ref().to_vec()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn build(self) -> Vec<u8> {
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

async fn make_session(request: &[u8]) -> Session {
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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

#[tokio::test]
async fn http_normalize_uses_http2_mode_when_version_is_http2() {
    // Arrange
    let request = RawHttpRequest::new("GET", "/grpc.Service/Method")
        .with_version("HTTP/2")
        .header("Host", "example.test")
        .build();
    let session = make_session(&request).await;
    let mut ctx = RequestCtx::empty();

    // Act
    let result = ctx.hydrate_from_session(&session);

    // Assert
    assert!(result.is_ok());
    assert!(ctx.is_http2());
    //todo add header specific test
    assert!(ctx.hydrated);
}

//-----------------------------------------------------------------------------
// Small utility methods
//-----------------------------------------------------------------------------
#[tokio::test]
async fn upstream_path_returns_override_when_set() {
    // Arrange
    let request = RawHttpRequest::new("GET", "/from-route")
        .with_version("HTTP/2")
        .header("Host", "example.test")
        .build();
    let session = make_session(&request).await;
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
        .with_version("HTTP/2")
        .header("Host", "example.test")
        .build();
    let session = make_session(&request).await;
    let mut ctx = RequestCtx::empty();
    let _ = ctx.hydrate_from_session(&session);

    // Act
    let result = ctx.upstream_path();

    // Assert
    assert_eq!(result, expected_path);
}

#[tokio::test]
async fn upstream_authority_return_none_when_not_set() {
    // Arrange
    let request = RawHttpRequest::new("GET", "/books?b=2&a=1")
        .header("Host", "example.test")
        .build();
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let session = make_session(&request).await;
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
    let request = RawHttpRequest::new("GET", "/hello?x=1")
        .header("Host", "example.test")
        .build();
    let session = make_session(&request).await;
    let mut ctx = RequestCtx::empty();
    let _ = ctx.hydrate_from_session(&session);

    // Act
    let uri_str = ctx.original_uri_string();

    // Assert
    assert_eq!(uri_str, "http://example.test/hello?x=1");
}
