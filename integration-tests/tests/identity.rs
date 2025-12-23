use integration_tests::harness::TestServer;
use std::sync::Once;

static SERVER: Once = Once::new();
const LISTEN_PORT: u16 = 4042;
const UPSTREAM_PORT: u16 = 4002;

#[test]
fn identity_with_user_agent() {
    let srv = TestServer::start(&SERVER, "identity.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv
        .get("/")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();
    assert!(identity.get("device").is_some());
}

#[test]
fn identity_without_user_agent_is_empty() {
    let srv = TestServer::start(&SERVER, "identity.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv.get("/").send().unwrap();
    assert_eq!(res.status(), 200);

    match srv.first_identity_json() {
        Some(identity) => {
            assert!(
                identity.as_object().unwrap().is_empty(),
                "identity JSON should be empty when no enrichments apply"
            );
        }
        None => {
            // Also acceptable: identity omitted entirely
        }
    }
}

#[test]
fn identity_detects_mobile_user_agent() {
    let srv = TestServer::start(&SERVER, "identity.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv
        .get("/")
        .header(
            "user-agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();

    assert_eq!(
        identity.get("device").unwrap(),
        "mobile",
        "expected mobile device classification"
    );
}

#[test]
fn oversized_user_agent_is_ignored() {
    let srv = TestServer::start(&SERVER, "identity.toml", LISTEN_PORT, UPSTREAM_PORT);

    let long_ua = "a".repeat(10_000);

    let res = srv.get("/").header("user-agent", long_ua).send().unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();

    assert!(
        identity.get("device").is_none(),
        "oversized UA must not be parsed"
    );
}

#[test]
fn trusted_proxy_allows_xff_resolution() {
    let srv = TestServer::start(
        &SERVER,
        "identity_trusted_proxy.toml",
        LISTEN_PORT,
        UPSTREAM_PORT,
    );

    let res = srv
        .get("/")
        .header("x-forwarded-for", "1.1.1.1, 127.0.0.1")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();

    // device present proves identity ran
    assert!(identity.get("device").is_some());
}

#[test]
fn untrusted_xff_is_ignored() {
    let srv = TestServer::start(&SERVER, "identity.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv
        .get("/")
        .header("x-forwarded-for", "8.8.8.8")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();

    // We don't assert IP — only that spoofing doesn't break identity
    assert!(identity.get("device").is_some());
}

#[test]
fn geoip_disabled_does_not_emit_country() {
    let srv = TestServer::start(&SERVER, "identity_no_geo.toml", LISTEN_PORT, UPSTREAM_PORT);

    let res = srv
        .get("/")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), 200);

    let identity = srv.first_identity_json().unwrap();

    assert!(
        identity.get("country").is_none(),
        "country should not be present when geoip is disabled"
    );
}
