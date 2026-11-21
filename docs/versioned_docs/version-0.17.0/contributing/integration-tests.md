---
title: Integration Tests
---

This page covers standard integration tests in the `crates/snakeway-tests` crate.
These tests spin up a real Snakeway server process and exercise it over real network connections.

For byte-level protocol tests, see [HTTP Replay Tests](http-replay-tests.md) instead.

## Where tests live

Integration tests are Rust test files inside `crates/snakeway-tests/tests/`.
The organizing rule: **one subdirectory per feature area, each with a `mod.rs` that declares its test modules**.
Current areas include `proxy/`, `device/`, `net/`, `acme/`, `cli/`, `configuration/`, `otel/`, `traffic/`, and `http_replay/`.
List the `tests/` directory for the authoritative set.

An illustrative excerpt:

```
crates/snakeway-tests/tests/
  proxy/
    basic_proxy.rs
    static_files.rs
    websocket.rs
    mod.rs
  device/
    identity.rs
    network_policy.rs
    mod.rs
  http_replay/        <- covered on the HTTP Replay Tests page
    ...
```

Put a new test in the file that matches its feature.
If no feature area fits, create a new subdirectory with its own `mod.rs`.

## Setting up a test server

There are two ways to create a `TestServer`.

### Option A: ConfigBuilder (programmatic, preferred for new tests)

Build a `RuntimeConfig` entirely in Rust using the fluent `ConfigBuilder` API, then hand it to `TestServer`:

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

| Helper                                     | What it builds                              |
|--------------------------------------------|----------------------------------------------|
| `minimal_http_runtime_config()`            | Plain HTTP listener and HTTP upstream        |
| `minimal_ws_runtime_config()`              | Plain HTTP listener and WebSocket upstream   |
| `minimal_grpc_runtime_config()`            | TLS listener and gRPC upstream               |
| `minimal_static_file_runtime_config()`     | Static file serving (no upstream)            |
| `minimal_https_runtime_config_with_acme()` | TLS listener with ACME automation            |

`ConfigBuilder` methods chain as needed.
Device and filter methods take a spec struct, with values wrapped in `Located::detached` since they have no source file:

```rust
ConfigBuilder::default()
    .with_http_ingress()
    .with_connection_filter(NetworkConnectionFilterSpec {
        cidr: Located::detached(CidrSpec {
            allow: vec![Located::detached("127.0.0.1/32".to_string())],
            deny: vec![],
        }),
        ip_family: Located::detached(IpFamilySpec {
            ipv4: Located::detached(true),
            ipv6: Located::detached(true),
        }),
        on_no_peer_addr: Located::detached("deny".to_string()),
    })
    .build()
```

Other builder methods follow the same shape, for example `with_request_filter(spec)`, `with_identity_device(spec)`, and `with_static_file_ingress(...)`.
See `crates/snakeway-tests/src/conf/builder/` for the full API.

### Option B: Fixture directory (for testing the HCL config loader)

Pass the name of a config fixture directory under `crates/snakeway-tests/fixtures/config/`:

```rust
let srv = TestServer::start_with_http_upstream("basic");
```

This loads a real HCL config from `fixtures/config/basic/`.
Use it mainly to verify the config loading path itself.

## Making requests

`TestServer` exposes a pre-configured `reqwest::blocking::Client` with convenience methods:

```rust
srv.get("/path")     // GET  with correct Host header
srv.post("/path")    // POST with correct Host header
srv.put("/path")     // PUT  with correct Host header
srv.delete("/path")  // DELETE with correct Host header
```

All methods return a `reqwest::blocking::RequestBuilder`, so you can chain headers and bodies before calling `.send()`.

For TLS endpoints use `srv.https_url()` and build your own client with the test CA cert:

```rust
let client = reqwest::blocking::Client::builder()
    .danger_accept_invalid_certs(true) // or pin the test CA
    .build()
    .unwrap();
let res = client.get(srv.https_url()).send().unwrap();
```

## Async protocols (WebSocket, gRPC)

WebSocket and gRPC tests are `#[test]` functions (not async) that create a Tokio runtime internally:

```rust
#[test]
fn websocket_echo_is_proxied() {
    // Arrange
    let mut cfg = minimal_ws_runtime_config();
    let srv = TestServer::start_ws_upstream_with_config(&mut cfg);
    let url = format!(
        "ws://{}{}",
        srv.base_url().as_str().strip_prefix("http://").unwrap(),
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

## Test structure and the AAA pattern

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

## Test naming

Use the same convention as unit tests: plain-English `snake_case` sentences describing the behavior.

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

## Useful constants

Import from `integration::constants`:

```rust
use integration::constants::{
    HTTP_RESPONSE_BODY,   // "hello world", the expected plain upstream body
    ROUTE_PATH_API,       // "/api"
    ROUTE_PATH_WS,        // "/ws"
    TEST_HOST,            // "snakeway.test"
    CERT_ORIGIN_CA_PEM,   // path to the test CA cert
};
```

## Running integration tests

```bash
# Full integration test run (starts Docker for ACME, generates certs)
just integration-test

# Run directly with nextest (assumes certs already exist)
cargo nextest run -p snakeway-tests

# Run a specific test
cargo nextest run -p snakeway-tests -E 'test(serves_index_html_from_static_dir)'
```

:::note
The integration test suite requires Docker for the ACME tests (Pebble CA).
For everything except ACME tests you can skip `just fetch-pebble-ca` and run `cargo nextest run -p snakeway-tests` directly after generating TLS certs once.
:::
