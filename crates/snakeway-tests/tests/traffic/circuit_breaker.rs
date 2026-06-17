use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::{
    CircuitBreakerSpec, RuntimeConfig, ServiceRouteSpec, ServiceSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::server::admin_client;
use snakeway_tests::harness::upstream::{
    start_http_upstream, start_http_upstream_that_returns_5xx, start_slow_http_upstream,
};
use std::time::Duration;

/// Extract the first upstream port from a patched RuntimeConfig.
fn extract_upstream_port(cfg: &RuntimeConfig) -> u16 {
    let svc = cfg.services.values().next().expect("no services in config");
    let url: url::Url = svc.tcp_upstreams[0]
        .url
        .parse()
        .expect("invalid upstream URL");
    url.port().expect("no port in upstream URL")
}

fn parse_upstream_circuit_states(json: &serde_json::Value) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(services) = json.get("services").and_then(|s| s.as_object()) {
        for (_svc, upstreams) in services {
            if let Some(upstreams) = upstreams.as_object() {
                for (endpoint, view) in upstreams {
                    let state = view
                        .get("circuit")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    result.push((endpoint.clone(), state));
                }
            }
        }
    }
    result
}

/// With healthy upstreams and successful requests, the circuit breaker
/// should remain in the Closed state.
#[test]
fn circuit_breaker_starts_closed() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            circuit_breaker: Some(Located::detached(CircuitBreakerSpec {
                enable_auto_recovery: Located::detached(true),
                failure_threshold: Located::detached(3),
                open_duration_milliseconds: Located::detached(5000),
                half_open_max_requests: Located::detached(1),
                success_threshold: Located::detached(2),
                count_http_5xx_as_failure: Located::detached(true),
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
    for _ in 0..5 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let states = parse_upstream_circuit_states(&json);

    // Assert
    assert!(!states.is_empty(), "should have circuit breaker state data");
    for (endpoint, state) in &states {
        assert_eq!(
            state, "closed",
            "circuit for {endpoint} should be Closed after successful requests"
        );
    }
}

/// When an upstream is unreachable (no listener on the allocated port),
/// consecutive connection failures should trip the circuit breaker to
/// Open. The proxy returns 502 for each failed connection attempt.
#[test]
fn circuit_breaker_trips_open_after_connection_failures() {
    // Arrange: use a no-op upstream function so the allocated port has
    // nothing listening on it. The proxy will get connection refused.
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            circuit_breaker: Some(Located::detached(CircuitBreakerSpec {
                enable_auto_recovery: Located::detached(true),
                failure_threshold: Located::detached(2),
                open_duration_milliseconds: Located::detached(30000),
                half_open_max_requests: Located::detached(1),
                success_threshold: Located::detached(1),
                count_http_5xx_as_failure: Located::detached(true),
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

    let srv = TestServer::start_with_config(&mut cfg, |_port| {
        // Intentionally do nothing: no upstream listener started.
        // The proxy will fail to connect on every request.
    });
    let admin = admin_client();

    // Act: send requests that will all fail (connection refused),
    // then poll the admin API until the circuit trips open.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let states = parse_upstream_circuit_states(&json);

        assert!(!states.is_empty(), "should have circuit breaker state data");

        if states.iter().any(|(_, state)| state == "open") {
            break;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "circuit did not trip Open within 5 seconds; last states: {states:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// After the circuit trips Open and the cooldown expires, the circuit
/// transitions to HalfOpen. Successful probe requests in half-open state
/// must close the circuit, restoring normal traffic flow.
#[test]
fn circuit_breaker_recovers_through_half_open_to_closed() {
    // Arrange: short cooldown so the test completes quickly.
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            circuit_breaker: Some(Located::detached(CircuitBreakerSpec {
                enable_auto_recovery: Located::detached(true),
                failure_threshold: Located::detached(2),
                open_duration_milliseconds: Located::detached(500),
                half_open_max_requests: Located::detached(2),
                success_threshold: Located::detached(2),
                count_http_5xx_as_failure: Located::detached(true),
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

    // Start with no upstream to force connection failures.
    let srv = TestServer::start_with_config(&mut cfg, |_port| {});
    let upstream_port = extract_upstream_port(&cfg);
    let admin = admin_client();

    // Trip circuit to Open via connection failures.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let states = parse_upstream_circuit_states(&json);
        if states.iter().any(|(_, s)| s == "open") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "circuit did not trip Open; states: {states:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    // Start a real upstream so half-open probes succeed.
    start_http_upstream(upstream_port);

    // Wait for the open_duration cooldown to expire.
    std::thread::sleep(Duration::from_millis(700));

    // Act: send requests. The circuit should transition HalfOpen -> Closed
    // after success_threshold (2) successful probes.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    // Assert: poll until circuit is closed.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let states = parse_upstream_circuit_states(&json);
        if states.iter().all(|(_, s)| s == "closed") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "circuit did not recover to Closed; states: {states:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Under concurrent load with a slow upstream, the circuit breaker must
/// remain Closed when all requests succeed. This verifies that concurrent
/// in-flight requests do not spuriously trip the circuit.
#[test]
fn circuit_breaker_stays_closed_under_concurrent_successful_load() {
    // Arrange: high failure threshold so successful requests never trip it.
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            circuit_breaker: Some(Located::detached(CircuitBreakerSpec {
                enable_auto_recovery: Located::detached(true),
                failure_threshold: Located::detached(10),
                open_duration_milliseconds: Located::detached(5000),
                half_open_max_requests: Located::detached(1),
                success_threshold: Located::detached(1),
                count_http_5xx_as_failure: Located::detached(true),
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

    // Slow upstream holds connections for 200ms, creating real concurrency.
    let srv = TestServer::start_with_config(&mut cfg, start_slow_http_upstream);
    let admin = admin_client();
    let base_url = srv.base_url();

    // Act: send concurrent requests.
    std::thread::scope(|s| {
        for _ in 0..10 {
            let url = base_url.join("/api").unwrap();
            s.spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                let res = client.get(url).header("Host", TEST_HOST).send().unwrap();
                assert_eq!(res.status(), StatusCode::OK);
            });
        }
    });

    // Assert: circuit must still be Closed.
    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let states = parse_upstream_circuit_states(&json);

    assert!(!states.is_empty(), "should have circuit breaker state data");
    for (endpoint, state) in &states {
        assert_eq!(
            state, "closed",
            "circuit for {endpoint} should remain Closed after concurrent successful requests"
        );
    }
}

/// When `count_http_5xx_as_failure` is enabled, HTTP 500 responses from
/// the upstream must count as failures and trip the circuit to Open.
#[test]
fn circuit_breaker_trips_open_on_5xx_responses() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            circuit_breaker: Some(Located::detached(CircuitBreakerSpec {
                enable_auto_recovery: Located::detached(true),
                failure_threshold: Located::detached(2),
                open_duration_milliseconds: Located::detached(30000),
                half_open_max_requests: Located::detached(1),
                success_threshold: Located::detached(1),
                count_http_5xx_as_failure: Located::detached(true),
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

    let srv = TestServer::start_with_config(&mut cfg, start_http_upstream_that_returns_5xx);
    let admin = admin_client();

    // Act: send requests that return 500.
    for _ in 0..5 {
        let _ = srv.get("/api").send();
    }

    // Assert: poll until circuit trips to Open.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let resp = admin
            .get(format!("{}/admin/upstreams", srv.admin_url()))
            .send()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
        let states = parse_upstream_circuit_states(&json);

        assert!(!states.is_empty(), "should have circuit breaker state data");

        if states.iter().any(|(_, state)| state == "open") {
            break;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "circuit did not trip Open after 5xx responses; states: {states:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
