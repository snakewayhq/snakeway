use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway::testing_api::conf::types::{ServiceRouteSpec, ServiceSpec};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{
    TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY, UPSTREAM_PORT_TERTIARY,
};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::server::admin_client;
use std::time::Duration;

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

fn build_lb_config(strategy: &str) -> snakeway::testing_api::conf::types::RuntimeConfig {
    ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            load_balancing_strategy: Located::detached(strategy.to_string()),
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
        .build()
}

/// Failover sends all requests to the first healthy upstream.
#[test]
fn failover_sends_all_to_first_upstream() {
    // Arrange
    let mut cfg = build_lb_config("failover");
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
    let counts = parse_upstream_request_counts(&json);

    // Assert
    assert_eq!(counts.len(), 2, "should have 2 upstreams");

    let max_requests = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
    let min_requests = counts.iter().map(|(_, c)| *c).min().unwrap_or(0);

    assert_eq!(
        max_requests, 5,
        "failover should send all requests to one upstream"
    );
    assert_eq!(
        min_requests, 0,
        "failover should not use the second upstream"
    );
}

/// Round-robin distributes requests across upstreams.
#[test]
fn round_robin_distributes_across_upstreams() {
    // Arrange
    let mut cfg = build_lb_config("round_robin");
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act
    for _ in 0..10 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_request_counts(&json);

    // Assert
    for (endpoint, total) in &counts {
        assert!(
            *total > 0,
            "round-robin: upstream {endpoint} should have received at least 1 request, got {total}"
        );
    }
}

/// Request-pressure selects the upstream with the fewest active requests.
/// To exercise this, we use a slow upstream (200 ms delay) and send
/// concurrent requests so that active connection counts diverge between
/// upstreams. Both upstreams should receive traffic, unlike Failover
/// where only the first gets requests.
#[test]
fn request_pressure_distributes_under_concurrent_load() {
    // Arrange
    let mut cfg = build_lb_config("request_pressure");
    let srv = TestServer::start_with_config(
        &mut cfg,
        snakeway_tests::harness::upstream::start_slow_http_upstream,
    );
    let admin = admin_client();
    let base_url = srv.base_url();

    // Act -- send concurrent requests so the slow upstream holds connections
    // open long enough for the load balancer to see active counts > 0.
    std::thread::scope(|s| {
        for _ in 0..10 {
            let url = base_url.join("/api").unwrap();
            s.spawn(move || {
                let client = Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                let res = client.get(url).header("Host", TEST_HOST).send().unwrap();
                assert_eq!(res.status(), StatusCode::OK);
            });
        }
    });

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_request_counts(&json);

    // Assert -- both upstreams should have received at least one request.
    // This distinguishes RequestPressure from Failover (which sends all
    // traffic to a single upstream).
    let total: u64 = counts.iter().map(|(_, c)| *c).sum();
    assert_eq!(
        total, 10,
        "request-pressure: all 10 requests should be accounted for"
    );
    for (endpoint, count) in &counts {
        assert!(
            *count > 0,
            "request-pressure: upstream {endpoint} should have received at least 1 request, got {count}"
        );
    }
}

/// Sticky-hash routes the same client consistently to the same upstream.
/// All requests from the same peer IP should go to a single upstream.
#[test]
fn sticky_hash_routes_same_client_consistently() {
    // Arrange
    let mut cfg = build_lb_config("sticky_hash");
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
    let counts = parse_upstream_request_counts(&json);

    // Assert
    let max_requests = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
    let min_requests = counts.iter().map(|(_, c)| *c).min().unwrap_or(0);

    assert_eq!(
        max_requests, 5,
        "sticky-hash: all requests from same client should go to one upstream"
    );
    assert_eq!(
        min_requests, 0,
        "sticky-hash: the other upstream should receive 0 requests"
    );
}

/// Random distributes requests across upstreams. With 100 requests and
/// 2 upstreams, the probability of all going to one is negligible
/// (2 * 0.5^100 ~ 1.6e-30).
#[test]
fn random_distributes_across_upstreams() {
    // Arrange
    let mut cfg = build_lb_config("random");
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act
    for _ in 0..100 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_request_counts(&json);

    // Assert
    for (endpoint, total) in &counts {
        assert!(
            *total > 0,
            "random: upstream {endpoint} should have received at least 1 request out of 100, got {total}"
        );
    }
}

/// Round-robin must distribute across 3 upstreams, not just 2.
/// This verifies the load balancer generalizes beyond the common
/// 2-upstream test setup.
#[test]
fn round_robin_distributes_across_3_upstreams() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            load_balancing_strategy: Located::detached("round_robin".to_string()),
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
                Located::detached(ConfigBuilder::make_tcp_upstream(
                    UPSTREAM_PORT_TERTIARY,
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
    for _ in 0..12 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_request_counts(&json);

    // Assert
    assert_eq!(counts.len(), 3, "should have 3 upstreams");
    for (endpoint, total) in &counts {
        assert!(
            *total > 0,
            "round-robin 3-upstream: {endpoint} should have received at least 1 request, got {total}"
        );
    }
}

/// Sticky-hash must be deterministic: two separate batches of requests
/// from the same client IP must route to the same upstream. The hash
/// uses fixed seeds, so the result should be stable within a server
/// lifetime.
#[test]
fn sticky_hash_is_deterministic_across_batches() {
    // Arrange
    let mut cfg = build_lb_config("sticky_hash");
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act: first batch of 5 requests.
    for _ in 0..5 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts_batch1 = parse_upstream_request_counts(&json);

    // Record which upstream got all 5 requests.
    let first_target: Vec<_> = counts_batch1
        .iter()
        .filter(|(_, c)| *c == 5)
        .map(|(e, _)| e.clone())
        .collect();
    assert_eq!(
        first_target.len(),
        1,
        "sticky-hash: all 5 requests should go to one upstream; got: {counts_batch1:?}"
    );

    // Act: second batch of 5 more requests.
    for _ in 0..5 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts_batch2 = parse_upstream_request_counts(&json);

    // Assert: the same upstream should now have all 10 requests.
    let second_target: Vec<_> = counts_batch2
        .iter()
        .filter(|(_, c)| *c == 10)
        .map(|(e, _)| e.clone())
        .collect();
    assert_eq!(
        second_target.len(),
        1,
        "sticky-hash: all 10 requests should go to the same upstream; got: {counts_batch2:?}"
    );
    assert_eq!(
        first_target[0], second_target[0],
        "sticky-hash: second batch must route to the same upstream as the first batch"
    );
}
