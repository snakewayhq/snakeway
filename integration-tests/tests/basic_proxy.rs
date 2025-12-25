use integration_tests::harness::TestServer;
use reqwest::StatusCode;

#[test]
fn should_proxy_to_upstream() {
    let srv = TestServer::start("basic");

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
