mod common;

use std::sync::Once;

static SERVER: Once = Once::new();
static CONFIG: &str = "identity.toml";

/// Identity device should resolve client IP from peer socket
#[test]
fn identity_resolves_client_ip_without_xff() {
    // Arrange
    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, CONFIG);

        // Act
        let res = reqwest::blocking::get("http://127.0.0.1:4041/api").expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    assert!(
        logs.contains("\"identity\""),
        "expected identity object in logs"
    );

    assert!(
        logs.contains("\"country\""),
        "expected country field from identity"
    );
}

/// X-Forwarded-For should be ignored if peer is not trusted
#[test]
fn untrusted_xff_is_ignored() {
    // Arrange
    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, CONFIG);

        let client = reqwest::blocking::Client::new();
        let res = client
            .get("http://127.0.0.1:4041/api")
            .header("x-forwarded-for", "8.8.8.8")
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    // We expect identity, but NOT proxy chain usage
    assert!(
        !logs.contains("8.8.8.8"),
        "untrusted XFF should not influence identity"
    );
}

/// Trusted proxies should allow XFF resolution
#[test]
fn trusted_proxy_allows_xff_resolution() {
    // Arrange
    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, CONFIG);

        let client = reqwest::blocking::Client::new();
        let res = client
            .get("http://127.0.0.1:4041/api")
            .header("x-forwarded-for", "1.1.1.1, 10.0.0.1")
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    assert!(
        logs.contains("1.1.1.1"),
        "expected client IP from XFF when proxy is trusted"
    );
}

/// User-Agent parsing should populate identity.ua
#[test]
fn identity_parses_user_agent() {
    // Arrange
    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, CONFIG);

        let client = reqwest::blocking::Client::new();
        let res = client
            .get("http://127.0.0.1:4041/api")
            .header(
                "user-agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
            )
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    assert!(
        logs.contains("\"device\""),
        "expected device field in identity"
    );
}

/// Excessively long User-Agent headers should be ignored
#[test]
fn oversized_user_agent_is_ignored() {
    // Arrange
    let long_ua = "a".repeat(10_000);

    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, CONFIG);

        let client = reqwest::blocking::Client::new();
        let res = client
            .get("http://127.0.0.1:4041/api")
            .header("user-agent", long_ua)
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    assert!(
        !logs.contains("\"device\""),
        "oversized UA should not be parsed or logged"
    );
}

/// GeoIP should be disabled cleanly when not configured
#[test]
fn geoip_disabled_does_not_break_identity() {
    // Arrange
    common::start_upstream();
    let logs = common::capture_logs(|| {
        common::start_server(&SERVER, "identity_no_geo.toml");

        let res = reqwest::blocking::get("http://127.0.0.1:4041/api").expect("request failed");

        assert_eq!(res.status(), 200);
    });

    // Assert
    assert!(
        !logs.contains("\"country\""),
        "geo fields should not appear when geoip is disabled"
    );
}
