//! Shared constants for integration-test configuration and assertions.
//!
//! This module is the single source of truth for all literal values that appear
//! in more than one place across `src/` and `tests/`. Changing a value here
//! propagates everywhere automatically.

// ---------------------------------------------------------------------------
// HTTP Replay
// ---------------------------------------------------------------------------

/// Path to the test fixture directory.
pub const FIXTURES_HTTP_DIR: &str = "fixtures/http";
pub const HTTP_REPLAY_OK_RESPONSE: &str = "200 OK";

// ---------------------------------------------------------------------------
// Network / addressing
// ---------------------------------------------------------------------------

/// The canonical test hostname used as the `Host` header, SNI, and route host
/// matcher throughout every integration test.
pub const TEST_HOST: &str = "snakeway.test";

/// Default listener port used in builder helpers (`make_bind`, `make_bind_with_acme`).
/// Always patched to a dynamic free port by the test harness at startup.
pub const DEFAULT_LISTENER_PORT: u16 = 8080;

/// Default admin listener port, to distinguish between non-admin listeners during config validation.
/// Always patched to a dynamic free port by the test harness at startup.
pub const DEFAULT_ADMIN_LISTENER_PORT: u16 = 8443;

/// Primary placeholder upstream port used when building service specs.
/// Always patched to a dynamic free port by the test harness at startup.
pub const UPSTREAM_PORT_PRIMARY: u16 = 9000;

/// Secondary placeholder upstream port used when building service specs.
/// Always patched to a dynamic free port by the test harness at startup.
pub const UPSTREAM_PORT_SECONDARY: u16 = 9001;

// ---------------------------------------------------------------------------
// Route paths
// ---------------------------------------------------------------------------

/// HTTP path prefix for the default service route.
pub const ROUTE_PATH_API: &str = "/api";

/// HTTP path for the WebSocket upgrade route.
pub const ROUTE_PATH_WS: &str = "/ws";

/// Full gRPC method path for the Greeter service used in proxy tests.
pub const ROUTE_PATH_GRPC: &str = "/helloworld.Greeter/SayHello";

// ---------------------------------------------------------------------------
// TLS / certificate file paths (relative to CARGO_MANIFEST_DIR, no leading ./)
// ---------------------------------------------------------------------------

/// Server TLS certificate (PEM), signed by the test CA.
pub const CERT_SERVER_PEM: &str = "certs/server.pem";

/// Server TLS private key (PEM).
pub const CERT_SERVER_KEY: &str = "certs/server.key";

/// Origin CA certificate used to verify upstream/client TLS connections.
pub const CERT_ORIGIN_CA_PEM: &str = "certs/origin-ca.pem";

/// Pebble (ACME test CA) root certificate.
pub const CERT_PEBBLE_CA_PEM: &str = "certs/pebble-ca.pem";

// ---------------------------------------------------------------------------
// ACME / TLS-automation paths and config
// ---------------------------------------------------------------------------

/// ACME order store directory (relative to CARGO_MANIFEST_DIR).
pub const ACME_ORDERS_DIR: &str = "acme/orders/";

/// ACME certificate store directory (relative to CARGO_MANIFEST_DIR).
pub const ACME_CERTS_DIR: &str = "acme/certs/";

/// ACME directory URL pointing at the local Pebble test server.
pub const ACME_DIRECTORY_URL: &str = "https://localhost:14000/dir";

/// Contact e-mail address registered with the ACME server in test configs.
pub const ACME_CONTACT_EMAIL: &str = "barryallen@example.com";

// ---------------------------------------------------------------------------
// Mock HTTP upstream response
// ---------------------------------------------------------------------------

/// Plain-text body returned by the mock plain-HTTP upstream.
/// Used in assertions: `assert_eq!(body, HTTP_RESPONSE_BODY)`.
pub const HTTP_RESPONSE_BODY: &str = "hello world";

/// Full raw HTTP/1.1 response written by the mock plain-HTTP upstream.
///
/// The `Content-Length` value (11) must stay in sync with the byte length of
/// `HTTP_RESPONSE_BODY`. If `HTTP_RESPONSE_BODY` ever changes, update both
/// this constant and the `Content-Length` digit accordingly.
pub const HTTP_UPSTREAM_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world";
