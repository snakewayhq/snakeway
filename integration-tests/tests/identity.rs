use integration_tests::harness::TestServer;
use reqwest::StatusCode;

/// Baseline: identity device runs when a User-Agent is present
#[test]
fn identity_with_user_agent() {
    let srv = TestServer::start("identity");

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

/// Identity should not break requests without a User-Agent
#[test]
fn identity_without_user_agent() {
    let srv = TestServer::start("identity");

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Oversized User-Agent headers should be ignored safely
#[test]
fn oversized_user_agent_is_ignored() {
    let srv = TestServer::start("identity");

    let long_ua = "a".repeat(10_000);

    let res = srv
        .get("/api")
        .header("user-agent", long_ua)
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Mobile User-Agent should not crash identity parsing
#[test]
fn mobile_user_agent_is_handled() {
    let srv = TestServer::start("identity");

    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Untrusted X-Forwarded-For headers must not affect request handling
#[test]
fn untrusted_xff_is_ignored() {
    let srv = TestServer::start("identity");

    let res = srv
        .get("/api")
        .header("x-forwarded-for", "8.8.8.8")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Trusted proxy config should accept XFF without error
#[test]
fn trusted_proxy_allows_xff() {
    let srv = TestServer::start("identity_trusted_proxy");

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
    let srv = TestServer::start("identity_no_geo");

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
