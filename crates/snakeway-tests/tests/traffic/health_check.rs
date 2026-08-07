use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway::testing_api::conf::types::{
    HealthCheckSpec, RuntimeConfig, ServiceRouteSpec, ServiceSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::server::{admin_client, free_port};
use snakeway_tests::harness::upstream::{start_http_upstream, start_http_upstream_on};
use std::time::Duration;

fn extract_upstream_port(cfg: &RuntimeConfig) -> u16 {
    let svc = cfg.services.values().next().expect("no services in config");
    let url: url::Url = svc.tcp_upstreams[0]
        .url
        .parse()
        .expect("invalid upstream URL");
    url.port().expect("no port in upstream URL")
}

fn parse_upstream_health(json: &serde_json::Value) -> Vec<(String, bool)> {
    let mut result = Vec::new();
    if let Some(services) = json.get("services").and_then(|s| s.as_object()) {
        for (_svc, upstreams) in services {
            if let Some(upstreams) = upstreams.as_object() {
                for (endpoint, view) in upstreams {
                    let healthy = view
                        .get("health")
                        .and_then(|h| h.get("healthy"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    result.push((endpoint.clone(), healthy));
                }
            }
        }
    }
    result
}

/// When health checks are enabled and both upstreams are responding,
/// the admin API must report both as healthy.
#[test]
fn health_check_enabled_reports_healthy_for_working_upstreams() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            health_check: Some(Located::detached(HealthCheckSpec {
                enable: Located::detached(true),
                failure_threshold: Located::detached(3),
                unhealthy_cooldown_seconds: Located::detached(5),
            })),
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_PRIMARY,
                    false,
                )),
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_SECONDARY,
                    false,
                )),
            ],
            ..Default::default()
        }])
        .with_admin_ingress()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act
    for _ in 0..4 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let health = parse_upstream_health(&json);

    // Assert
    assert!(!health.is_empty(), "should have upstream health data");
    for (endpoint, healthy) in &health {
        assert!(
            *healthy,
            "upstream {endpoint} should be healthy when health checks are enabled and upstream is responding"
        );
    }
}

/// When health checks are disabled (the default), all upstreams are
/// assumed healthy regardless of actual state.
#[test]
fn health_check_disabled_reports_healthy_by_default() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            health_check: Some(Located::detached(HealthCheckSpec {
                enable: Located::detached(false),
                ..Default::default()
            })),
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_PRIMARY,
                    false,
                )),
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_SECONDARY,
                    false,
                )),
            ],
            ..Default::default()
        }])
        .with_admin_ingress()
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act
    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let health = parse_upstream_health(&json);

    // Assert
    assert!(!health.is_empty(), "should have upstream health data");
    for (endpoint, healthy) in &health {
        assert!(
            *healthy,
            "upstream {endpoint} should be healthy when health checks are disabled"
        );
    }
}

/// After an upstream becomes unhealthy due to consecutive failures, it
/// must recover to healthy once the cooldown expires and a successful
/// request is processed. Health checks are passive (driven by request
/// outcomes), so recovery requires a real request to succeed.
#[test]
fn unhealthy_upstream_recovers_after_cooldown_and_success() {
    // Arrange: single upstream, low thresholds, short cooldown.
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            health_check: Some(Located::detached(HealthCheckSpec {
                enable: Located::detached(true),
                failure_threshold: Located::detached(2),
                unhealthy_cooldown_seconds: Located::detached(1),
            })),
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![Located::detached(ConfigBuilder::make_tcp_upstream(
                UPSTREAM_PORT_PRIMARY,
                false,
            ))],
            ..Default::default()
        }])
        .with_admin_ingress()
        .build();

    // Start with no upstream to force failures.
    let srv = TestServer::start_with_config(&mut cfg, free_port);
    let upstream_port = extract_upstream_port(&cfg);
    let admin = admin_client();

    // Send requests that fail (connection refused) to trip health to unhealthy.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    // Poll until at least one upstream is unhealthy.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let health = parse_upstream_health(&json);
        if health.iter().any(|(_, h)| !h) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "upstream did not become unhealthy; health: {health:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    // Start the upstream so future requests succeed.
    start_http_upstream_on(upstream_port);

    // Wait for the unhealthy cooldown to expire.
    std::thread::sleep(Duration::from_millis(1200));

    // Act: send requests. After cooldown, the upstream gets a trial and
    // report_success() restores it to healthy.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    // Assert: poll until all upstreams are healthy again.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let health = parse_upstream_health(&json);
        if health.iter().all(|(_, h)| *h) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "upstream did not recover to healthy; health: {health:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn parse_upstream_request_counts(json: &serde_json::Value) -> Vec<(String, u64)> {
    let mut result = Vec::new();
    if let Some(services) = json.get("services").and_then(|s| s.as_object()) {
        for (_svc, upstreams) in services {
            if let Some(upstreams) = upstreams.as_object() {
                for (endpoint, view) in upstreams {
                    let total = view
                        .get("total_requests")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    result.push((endpoint.clone(), total));
                }
            }
        }
    }
    result
}

/// When one of two upstreams becomes unhealthy, the load balancer must
/// skip it and route all requests to the remaining healthy upstream.
#[test]
fn unhealthy_upstream_is_skipped_during_routing() {
    // Arrange: two upstreams, only the second has a real listener.
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            health_check: Some(Located::detached(HealthCheckSpec {
                enable: Located::detached(true),
                failure_threshold: Located::detached(2),
                unhealthy_cooldown_seconds: Located::detached(30),
            })),
            routes: vec![Located::detached(ServiceRouteSpec {
                hosts: vec![Located::detached(TEST_HOST.to_string())],
                path: Located::detached("/api".to_string()),
                ..Default::default()
            })],
            upstreams: vec![
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_PRIMARY,
                    false,
                )),
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_SECONDARY,
                    false,
                )),
            ],
            ..Default::default()
        }])
        .with_admin_ingress()
        .build();

    // Start a listener only on the second upstream port. The first
    // upstream port will have no listener (connection refused).
    let mut first_call = true;
    let srv = TestServer::start_with_config(&mut cfg, || {
        if std::mem::take(&mut first_call) {
            free_port()
        } else {
            start_http_upstream()
        }
    });
    let admin = admin_client();

    // Act: send enough requests for the first upstream to become
    // unhealthy, then send more so all subsequent traffic goes to
    // the healthy upstream.
    for _ in 0..10 {
        let _ = srv.get("/api").send();
    }

    // Allow health state to propagate.
    std::thread::sleep(Duration::from_millis(200));

    // Send a final batch that should all route to the healthy upstream.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    // Assert: the healthy upstream received the majority of requests.
    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_request_counts(&json);

    assert_eq!(counts.len(), 2, "should have 2 upstreams");

    let max_requests = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
    assert!(
        max_requests >= 10,
        "the healthy upstream should have received the bulk of requests; counts: {counts:?}"
    );
}
