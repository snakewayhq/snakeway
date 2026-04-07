use integration::conf::ConfigBuilder;
use integration::constants::{TEST_HOST, UPSTREAM_PORT_PRIMARY, UPSTREAM_PORT_SECONDARY};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway_core::testing_api::conf::types::{HealthCheckSpec, ServiceRouteSpec, ServiceSpec};
use std::time::Duration;

fn admin_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build admin client")
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
            health_check: Some(HealthCheckSpec {
                enable: true,
                failure_threshold: 3,
                unhealthy_cooldown_seconds: 5,
            }),
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
            health_check: Some(HealthCheckSpec {
                enable: false,
                ..Default::default()
            }),
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
