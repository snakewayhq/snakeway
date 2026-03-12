use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

#[test]
fn should_load_config_files() {
    let srv = TestServer::start_with_http_upstream("basic");

    let res = srv.get("/api").send().expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
