---
title: HTTP Replay Tests
---

HTTP replay tests send **raw, pre-recorded HTTP request bytes** over a plain TCP socket to a running Snakeway instance and assert on the raw response string.
They are ideal for verifying protocol-level behavior that cannot be expressed with a high-level HTTP client: malformed requests, smuggling attempts, browser-specific header quirks, large cookies, and hop-by-hop header handling.

## How the Harness Works

```
crates/snakeway-tests/
  fixtures/
    http/                   <- raw .http request fixture files
      headers/
        duplicate_header.http
        hop_by_hop.http
      smuggling/
        cl_te.http
        te_cl.http
        dual_content_length.http
      cookies/
        large_cookie.http
      browsers/
        chrome_navigation.http
        chrome_fetch.http
        firefox_navigation.http
  src/
    harness/
      replay_http.rs        <- replay_http_fixture(path, port) implementation
  tests/
    http_replay/
      mod.rs                <- shared replay_fixture() helper + module declarations
      headers.rs
      smuggling.rs
      cookies.rs
      browsers.rs
```

`replay_http_fixture(path, port)` (in `src/harness/replay_http.rs`) does the following:

1. Reads the raw bytes from `fixtures/http/<path>`.
2. Normalizes line endings to `\r\n` and ensures the request ends with `\r\n\r\n`.
3. Opens a plain TCP connection to `TEST_HOST:port`.
4. Writes the raw bytes.
5. Reads the response until the socket closes or a 10-second timeout fires.
6. Returns the response as a `String`.

The module-level `replay_fixture(path)` helper in `tests/http_replay/mod.rs` handles server startup, so individual tests need only one line of setup:

```rust
pub fn replay_fixture(path: &str) -> String {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    srv.replay_http_fixture(path)
}
```

## Writing a Fixture File

Create a plain text file in `crates/snakeway-tests/fixtures/http/<category>/my_request.http`.
The file contains a raw HTTP/1.1 request exactly as it would appear on the wire.
Line endings can be `\n` because the harness normalizes them to `\r\n` automatically.

```http
GET /api HTTP/1.1
Host: snakeway.test:8080
Connection: keep-alive
User-Agent: test-client
Accept: text/html
Accept: application/json
```

For requests with bodies, include the body after the blank line:

```http
POST /api HTTP/1.1
Host: snakeway.test:8080
Content-Length: 13
Transfer-Encoding: chunked

0

GET /api HTTP/1.1
Host: snakeway.test:8080
```

Keep fixture files focused on the one property being tested.
A fixture should represent a single, specific HTTP scenario.

## Writing a Replay Test

Add the test to the appropriate file under `tests/http_replay/`, or create a new file and declare it in `tests/http_replay/mod.rs`.

```rust
// tests/http_replay/headers.rs
use super::replay_fixture;
use integration::constants::HTTP_REPLAY_OK_RESPONSE;

#[test]
fn duplicate_headers_should_proxy() {
    let resp = replay_fixture("headers/duplicate_header.http");
    assert!(resp.contains(HTTP_REPLAY_OK_RESPONSE));
}
```

For **rejection tests** (requests that Snakeway should refuse or not forward):

```rust
#[test]
fn cl_te_smuggling_should_be_rejected() {
    let resp = replay_fixture("smuggling/cl_te.http");

    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "CL.TE smuggling attempt should not be accepted or proxied"
    );
}
```

## Useful Constants

```rust
use integration::constants::HTTP_REPLAY_OK_RESPONSE; // "200 OK"
```

`HTTP_REPLAY_OK_RESPONSE` (`"200 OK"`) is the canonical string to check for a successful proxy pass-through.
Its presence means the request reached the upstream and received a 200 response.
Its absence means Snakeway rejected or blocked the request.

## Declaring a New Test File

1. Create the file: `crates/snakeway-tests/tests/http_replay/my_category.rs`
2. Declare it in `crates/snakeway-tests/tests/http_replay/mod.rs`:

```rust
mod browsers;
mod cookies;
mod headers;
mod my_category;   // <- add this
mod smuggling;
```

## When to Use Replay Tests vs Standard Integration Tests

| Use HTTP replay when...                                                  | Use standard integration tests when...                  |
|--------------------------------------------------------------------------|---------------------------------------------------------|
| Testing protocol-level edge cases (smuggling, malformed headers)         | Testing feature behavior via normal HTTP verbs          |
| Verifying browser-specific header sets                                   | Testing response status codes, bodies, JSON             |
| Reproducing a bug that requires an exact byte-level request              | Testing WebSocket or gRPC protocol flows                |
| Testing request normalization (duplicate headers, hop-by-hop stripping)  | Testing configuration options and their effects         |
| The scenario cannot be expressed with a high-level HTTP client           | The test can be written with `srv.get()` / `srv.post()` |

## Adding a New Category

1. Create a directory: `crates/snakeway-tests/fixtures/http/my_category/`
2. Add fixture files: `crates/snakeway-tests/fixtures/http/my_category/scenario_name.http`
3. Create a test file: `crates/snakeway-tests/tests/http_replay/my_category.rs`
4. Declare it in `crates/snakeway-tests/tests/http_replay/mod.rs`

## Running Replay Tests

```bash
# Run only the http_replay test group
cargo nextest run -p snakeway-tests -E 'test(http_replay)'

# Run a specific replay test
cargo nextest run -p snakeway-tests -E 'test(cl_te_smuggling_should_be_rejected)'

# Run the full integration suite (includes replay tests)
just integration-test
```
