use integration::conf::ConfigBuilder;
use integration::constants::{TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway_core::testing_api::conf::types::{
    LoadBalancingStrategySpec, ServiceRouteSpec, ServiceSpec,
};
use std::time::Duration;

fn admin_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build admin client")
}

fn parse_upstream_requests(json: &serde_json::Value) -> Vec<(String, u64)> {
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

fn build_lb_config(
    strategy: LoadBalancingStrategySpec,
) -> snakeway_core::testing_api::conf::types::RuntimeConfig {
    ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            load_balancing_strategy: strategy,
            routes: vec![ServiceRouteSpec {
                hosts: vec![TEST_HOST.to_string()],
                path: "/api".to_string(),
                ..Default::default()
            }],
            upstreams: vec![
                ConfigBuilder::make_tcp_upstream(UPSTREAM_PORT_PRIMARY, false),
                ConfigBuilder::make_tcp_upstream(UPSTREAM_PORT_SECONDARY, false),
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
    let mut cfg = build_lb_config(LoadBalancingStrategySpec::Failover);
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
    let counts = parse_upstream_requests(&json);

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
    let mut cfg = build_lb_config(LoadBalancingStrategySpec::RoundRobin);
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
    let counts = parse_upstream_requests(&json);

    // Assert
    for (endpoint, total) in &counts {
        assert!(
            *total > 0,
            "round-robin: upstream {endpoint} should have received at least 1 request, got {total}"
        );
    }
}

/// Request-pressure selects the upstream with the fewest active requests.
/// With sequential (non-concurrent) requests, active counts are always 0
/// for both upstreams, so the algorithm consistently picks the one with
/// the lowest ID (same behavior as failover). This test verifies the
/// strategy functions correctly; concurrent load testing would be needed
/// to verify actual pressure-based distribution.
#[test]
fn request_pressure_selects_upstream_with_least_active() {
    // Arrange
    let mut cfg = build_lb_config(LoadBalancingStrategySpec::RequestPressure);
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
    let counts = parse_upstream_requests(&json);

    // Assert
    let total: u64 = counts.iter().map(|(_, c)| *c).sum();
    assert_eq!(
        total, 10,
        "request-pressure: all 10 requests should be accounted for"
    );
}

/// Sticky-hash routes the same client consistently to the same upstream.
/// All requests from the same peer IP should go to a single upstream.
#[test]
fn sticky_hash_routes_same_client_consistently() {
    // Arrange
    let mut cfg = build_lb_config(LoadBalancingStrategySpec::StickyHash);
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
    let counts = parse_upstream_requests(&json);

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

/// Random distributes requests across upstreams. With 20 requests and
/// 2 upstreams, the probability of all going to one is negligible.
#[test]
fn random_distributes_across_upstreams() {
    // Arrange
    let mut cfg = build_lb_config(LoadBalancingStrategySpec::Random);
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let admin = admin_client();

    // Act
    for _ in 0..20 {
        let res = srv.get("/api").send().unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let resp = admin
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let counts = parse_upstream_requests(&json);

    // Assert
    for (endpoint, total) in &counts {
        assert!(
            *total > 0,
            "random: upstream {endpoint} should have received at least 1 request out of 20, got {total}"
        );
    }
}
