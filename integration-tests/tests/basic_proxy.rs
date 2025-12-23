use integration_tests::harness::TestServer;
use std::sync::Once;

static SERVER: Once = Once::new();
const LISTEN_PORT: u16 = 4041;
const UPSTREAM_PORT: u16 = 4001;

#[test]
fn should_proxy_request() {
    let srv = TestServer::start(&SERVER, "basic.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv.get("/").send().unwrap();
    assert_eq!(res.status(), 200);
}
