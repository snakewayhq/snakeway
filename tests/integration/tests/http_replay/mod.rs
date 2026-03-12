mod browsers;
mod cookies;
mod headers;
mod smuggling;

use integration::conf::minimal_http_runtime_config;
use integration::harness::TestServer;

pub fn replay_fixture(path: &str) -> String {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    srv.replay_http_fixture(path)
}
