use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::{ConfigBuilder, minimal_http_runtime_config};
use snakeway_tests::harness::TestServer;

/// Identity device is optional. If not configured, the proxy should still work.
#[test]
fn identity_without_user_agent() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Identity device with a recognized user-agent string should enrich the request
/// and still proxy successfully.
#[test]
fn identity_with_user_agent() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// A mobile user-agent header should be handled without error.
#[test]
fn mobile_user_agent_is_handled() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (Linux; Android 10; SM-G973F) AppleWebKit/537.36",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// An oversized user-agent header should be accepted but potentially truncated.
/// It must not crash or reject.
#[test]
fn oversized_user_agent_is_ignored() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let big_ua = "a".repeat(4096);
    let res = srv.get("/api").header("user-agent", big_ua).send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// A request without X-Forwarded-For, even if trusted proxies are set, should
/// still succeed — the proxy should use the peer IP directly.
#[test]
fn untrusted_xff_is_ignored() {
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header("x-forwarded-for", "1.1.1.1, 2.2.2.2")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Trusted proxy config should accept XFF without error
#[test]
fn trusted_proxy_allows_xff() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header("x-forwarded-for", "1.1.1.1, 127.0.0.1")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// GeoIP-disabled config must not fail identity processing
#[test]
fn geoip_disabled_does_not_break_identity() {
    let mut id = ConfigBuilder::make_identity_device();
    id.enable_geoip = Located::detached(false);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(id)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
