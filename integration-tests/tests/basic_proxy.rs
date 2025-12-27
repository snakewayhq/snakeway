use integration_tests::harness::TestServer;
use reqwest::StatusCode;

#[test]
fn should_proxy_to_upstream() {
    let srv = TestServer::start_with_http_upstream("basic");

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
