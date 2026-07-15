//! Shared constants for integration-test configuration and assertions.
//!
//! This module is the single source of truth for all literal values that appear
//! in more than one place across `src/` and `tests/`. Changing a value here
//! propagates everywhere automatically.

// ---------------------------------------------------------------------------
// File-based configuration
// ---------------------------------------------------------------------------

/// Path to the config fixture directory.
pub const FIXTURES_CONFIG_DIR: &str = "fixtures/config";

// ---------------------------------------------------------------------------
// HTTP Replay
// ---------------------------------------------------------------------------

/// Path to the HTTP replay fixture directory.
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
pub const DEFAULT_LISTENER_PORT: i64 = 8080;

/// Default admin listener port, to distinguish between non-admin listeners during config validation.
/// Always patched to a dynamic free port by the test harness at startup.
pub const DEFAULT_ADMIN_LISTENER_PORT: i64 = 8443;

/// Primary placeholder upstream port used when building service specs.
/// Always patched to a dynamic free port by the test harness at startup.
pub const UPSTREAM_PORT_PRIMARY: i64 = 9000;

/// Secondary placeholder upstream port used when building service specs.
/// Always patched to a dynamic free port by the test harness at startup.
pub const UPSTREAM_PORT_SECONDARY: i64 = 9001;

/// Tertiary placeholder upstream port for tests requiring 3+ upstreams.
/// Always patched to a dynamic free port by the test harness at startup.
pub const UPSTREAM_PORT_TERTIARY: i64 = 9002;

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
// Admin API authentication fixtures
// ---------------------------------------------------------------------------

/// Path to the bearer-token file used by builder-generated admin listeners.
pub const ADMIN_TOKEN_FILE: &str = "certs/admin.tokens";

/// The first token in `ADMIN_TOKEN_FILE`. Tests authenticate by sending
/// `Authorization: Bearer {ADMIN_TOKEN}`.
pub const ADMIN_TOKEN: &str = "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04";

/// The second token in `ADMIN_TOKEN_FILE`. Used to exercise rotation: both
/// tokens must be accepted by the running proxy.
pub const ADMIN_TOKEN_ALT: &str =
    "7b4e19a2c5f8d3046e9b71c8a52f9e1d4c07bfa6e93d1c24b87a90fed362014c";

// ---------------------------------------------------------------------------
// ACME / TLS-automation paths and config
// ---------------------------------------------------------------------------

/// ACME directory URL pointing at the local Pebble test server.
pub const ACME_DIRECTORY_URL: &str = "https://localhost:14000/dir";

/// Contact e-mail address registered with the ACME server in test configs.
pub const ACME_CONTACT_EMAIL: &str = "barryallen@example.com";

pub const ACME_ORDER_DIR: &str = "orders";

pub const ACME_CERT_DIR: &str = "certs";

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

// ---------------------------------------------------------------------------
// WASM
// ---------------------------------------------------------------------------
pub const TEST_DEVICE_PATH: &str = "fixtures/wasm/test_device.wasm";
pub const TEST_JWT_DEVICE_PATH: &str = "fixtures/wasm/jwt_auth_device.wasm";
