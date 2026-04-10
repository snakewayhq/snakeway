use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::minimal_http_runtime_config;
use snakeway_tests::harness::TestServer;

#[test]
fn should_proxy_to_upstream() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
