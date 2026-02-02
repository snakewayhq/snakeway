use integration_tests::conf::{ConfigBuilder, minimal_http_runtime_config};
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

#[test]
fn should_proxy_to_upstream() {
    let mut cfg = ConfigBuilder::default()
        .with_connection_filtered_http_ingress()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
