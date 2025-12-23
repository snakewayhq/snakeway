use integration_tests::harness::TestServer;

#[test]
fn should_proxy_to_upstream() {
    let srv = TestServer::start("fixtures/basic.toml");

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), 200);
}
