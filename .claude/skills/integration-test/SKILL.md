# Skill: integration-test — Writing Standard Integration Tests

This skill covers how to write standard (non-HTTP-replay) integration tests in the
`tests/integration` crate. These tests spin up a real Snakeway server process and
exercise it over real network connections.

## Where Tests Live

Integration tests are Rust test files inside `tests/integration/tests/`, organised
into subdirectories by feature area:

```
tests/integration/tests/
  proxy/
    basic_proxy.rs
    static_files.rs
    websocket.rs
    grpc.rs
    config_validation.rs
    mod.rs
  acme/
    http01.rs
    mod.rs
  device/
    identity.rs
    network_policy.rs
    request_filter.rs
    request_rate_limiting.rs
    mod.rs
  cli/
    route_solve.rs
    mod.rs
  http_replay/        ← covered separately in the http-replay-test skill
    ...
```

Each subdirectory has a `mod.rs` that declares the test modules.
New feature areas get their own subdirectory.

## Setting Up a Test Server

There are two ways to create a `TestServer`:

### Option A — `ConfigBuilder` (programmatic, preferred for new tests)

Build a `RuntimeConfig` entirely in Rust using the fluent `ConfigBuilder` API,
then hand it to `TestServer`:

```rust
use integration::conf::ConfigBuilder;
use integration::harness::TestServer;

#[test]
fn my_test() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), reqwest::StatusCode::OK);
}
```

Common pre-built helpers in `integration::conf`:

| Helper                                     | What it builds                           |
|--------------------------------------------|------------------------------------------|
| `minimal_http_runtime_config()`            | Plain HTTP listener + HTTP upstream      |
| `minimal_ws_runtime_config()`              | Plain HTTP listener + WebSocket upstream |
| `minimal_grpc_runtime_config()`            | TLS listener + gRPC upstream             |
| `minimal_static_file_runtime_config()`     | Static file serving (no upstream)        |
| `minimal_https_runtime_config_with_acme()` | TLS listener with ACME automation        |

`ConfigBuilder` methods (chain as needed):

```rust
ConfigBuilder::default ()
.with_http_ingress()
.with_request_filter_device()
.with_connection_filter_cidr_deny_list( & ["192.168.1.0/24"])
.build()
```

### Option B — Fixture directory (for testing the HCL config loader)

Pass the name of a config fixture directory under `tests/integration/fixtures/config/`:

```rust
let srv = TestServer::start_with_http_upstream("basic");
```

This loads a real HCL config from `fixtures/config/basic/` and is mainly used to
verify the config loading path itself.

## Making Requests

`TestServer` exposes a pre-configured `reqwest::blocking::Client` with convenience methods:

```rust
srv.get("/path")     // GET  with correct Host header
srv.post("/path")    // POST with correct Host header
srv.put("/path")     // PUT  with correct Host header
srv.delete("/path")  // DELETE with correct Host header
```

All methods return a `reqwest::blocking::RequestBuilder` so you can chain headers,
bodies, etc. before calling `.send()`.

For TLS endpoints use `srv.https_url()` and build your own client with the test CA cert:

```rust
let client = reqwest::blocking::Client::builder()
.danger_accept_invalid_certs(true) // or pin the test CA
.build()
.unwrap();
let res = client.get(srv.https_url()).send().unwrap();
```

## Async Protocols (WebSocket, gRPC)

WebSocket and gRPC tests are `#[test]` functions (not async) that create a Tokio
runtime internally:

```rust
#[test]
fn websocket_echo_is_proxied() {
    // Arrange
    let mut cfg = minimal_ws_runtime_config();
    let srv = TestServer::start_ws_upstream_with_config(&mut cfg);
    let url = format!(
        "ws://{}{}",
        srv.base_url().strip_prefix("http://").unwrap(),
        ROUTE_PATH_WS
    );

    // Act + Assert
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (mut socket, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("ws connect failed");

        socket
            .send(tokio_tungstenite::tungstenite::Message::Text("ping".into()))
            .await
            .unwrap();

        let msg = socket.next().await.unwrap().unwrap();
        assert_eq!(msg.into_text().unwrap(), "ping");
    });
}
```

## Test Structure and AAA Pattern

For short, single-assertion tests the AAA sections may be implicit:

```rust
#[test]
fn should_proxy_to_upstream() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
```

For longer or multi-step tests use explicit `//---` Arrange / Act / Assert banners:

```rust
#[test]
fn should_issue_certificate_via_http01_and_serve_tls() {
    //-------------------------------------------------------------------------
    // Arrange
    //-------------------------------------------------------------------------
    let mut cfg = minimal_https_runtime_config_with_acme();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    //-------------------------------------------------------------------------
    // Act: wait for certificate issuance
    //-------------------------------------------------------------------------
    // ... polling loop ...

    //-------------------------------------------------------------------------
    // Assert: verify real TLS handshake works
    //-------------------------------------------------------------------------
    let res = https_client.get(srv.https_url()).send().expect("TLS request failed");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

## Test Naming

Same convention as unit tests: plain-English `snake_case` sentences describing the behaviour:

```
should_proxy_to_upstream
serves_index_html_from_static_dir
static_path_traversal_is_rejected
if_none_match_returns_304
```

Use doc comments (`///`) on tests that need more context:

```rust
/// Serves index.html from the configured static directory
#[test]
fn serves_index_html_from_static_dir() { ... }
```

## Useful Constants

Import from `integration::constants`:

```rust
use integration::constants::{
    HTTP_RESPONSE_BODY,   // "hello world" — expected plain upstream body
    ROUTE_PATH_API,       // "/api"
    ROUTE_PATH_WS,        // "/ws"
    TEST_HOST,            // "snakeway.test"
    CERT_ORIGIN_CA_PEM,   // path to the test CA cert
};
```

## Running Integration Tests

```bash
# Full integration test run (starts Docker for ACME, generates certs)
just integration-test

# Run directly with nextest (assumes certs already exist)
cargo nextest run -p integration

# Run a specific test
cargo nextest run -p integration -E 'test(serves_index_html_from_static_dir)'
```

> **Note:** The integration test suite requires Docker for ACME tests (Pebble CA).
> For everything except ACME tests you can skip `just fetch-pebble-ca` and run
> `cargo nextest run -p integration` directly after generating TLS certs once.
