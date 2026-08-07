mod browsers;
mod connection;
mod cookies;
mod encoding;
mod headers;
mod malformed;
mod methods;
mod security;
mod smuggling;
mod uri;

use snakeway_tests::conf::minimal_http_runtime_config;
use snakeway_tests::harness::TestServer;

pub fn replay_fixture(path: &str) -> String {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    srv.replay_http_fixture(path)
}

/// Replay a fixture against an upstream that echoes the request line it
/// received, so tests can assert on what the proxy forwarded.
pub fn replay_fixture_with_request_line_echo(path: &str) -> String {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_request_line_with_config(&mut cfg);

    srv.replay_http_fixture(path)
}
