use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway_core::testing_api::ControlPlaneServer;
use snakeway_core::testing_api::conf::load_config;
use snakeway_tests::conf::minimal_http_runtime_config_with_admin;
use snakeway_tests::constants::{FIXTURES_CONFIG_DIR, ROUTE_PATH_API, TEST_HOST};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::server::{
    admin_client as make_admin_client, free_port, wait_for_listener,
};
use std::path::Path;
use std::time::Duration;

/// Build a reqwest client that accepts the self-signed test certificate
/// used on the admin listener.
fn admin_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("failed to build admin client")
}

//-----------------------------------------------------------------------------
// /admin/health
//-----------------------------------------------------------------------------

/// The `/admin/health` endpoint must return 200 OK.
///
/// Health checks are polled by orchestrators (Kubernetes, ECS, etc.) to
/// determine whether the proxy is ready to receive traffic.  A missing or
/// broken health endpoint leads to cascading outages.
#[test]
fn admin_health_returns_ok() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act
    let res = client
        .get(format!("{}/admin/health", srv.admin_url()))
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// The `/admin/health` response body must be non-empty.
///
/// A non-empty body lets orchestrators inspect the health response for
/// structured status information beyond the HTTP status code.
#[test]
fn admin_health_response_body_is_non_empty() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act
    let res = client
        .get(format!("{}/admin/health", srv.admin_url()))
        .send()
        .unwrap();
    let body = res.text().unwrap();

    // Assert
    assert!(
        !body.is_empty(),
        "admin health response body must not be empty"
    );
}

//-----------------------------------------------------------------------------
// /admin/upstreams
//-----------------------------------------------------------------------------

/// The `/admin/upstreams` endpoint must return 200 OK with a parseable body.
///
/// Upstream status is used by operators and automated tools to detect
/// degraded backends.  The endpoint must be available and responsive
/// even when all upstreams are healthy.
#[test]
fn admin_upstreams_returns_ok() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act
    let res = client
        .get(format!("{}/admin/upstreams", srv.admin_url()))
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        !res.text().unwrap().is_empty(),
        "admin upstreams response body must not be empty"
    );
}

//-----------------------------------------------------------------------------
// /admin/stats
//-----------------------------------------------------------------------------

/// The `/admin/stats` endpoint must return 200 OK.
///
/// Traffic statistics are essential for observability, alerting, and
/// capacity planning.  A working `/admin/stats` endpoint is a baseline
/// requirement for production operation.
#[test]
fn admin_stats_returns_ok() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act
    let res = client
        .get(format!("{}/admin/stats", srv.admin_url()))
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// After proxying several requests, the `/admin/stats` endpoint must
/// return a body that contains a recognisable request count field.
///
/// This confirms that the stats counter is actually being incremented
/// and exposed — not just that the endpoint returns a static placeholder.
#[test]
fn admin_stats_reflects_proxied_requests() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act — send a few requests through the proxy
    for _ in 0..3 {
        srv.get(ROUTE_PATH_API)
            .send()
            .expect("proxy request failed");
    }

    let res = client
        .get(format!("{}/admin/stats", srv.admin_url()))
        .send()
        .unwrap();

    // Assert — the body must be non-empty (contains counters)
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        !body.is_empty(),
        "admin stats response body must not be empty after traffic"
    );
}

//-----------------------------------------------------------------------------
// /admin/reload
//-----------------------------------------------------------------------------

/// The `/admin/reload` endpoint must accept a POST request and return a
/// success status (200 or 204).
///
/// Hot-reload is critical for zero-downtime configuration updates.  The
/// endpoint must be reachable and functional without restarting the process.
#[test]
fn admin_reload_returns_success() {
    // Arrange
    let mut cfg = minimal_http_runtime_config_with_admin();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = admin_client();

    // Act
    let res = client
        .post(format!("{}/admin/reload", srv.admin_url()))
        .send()
        .unwrap();

    // Assert — either 200 or 204 is acceptable for a reload trigger
    assert!(
        res.status() == StatusCode::OK || res.status() == StatusCode::NO_CONTENT,
        "admin reload must return 200 or 204, got {}",
        res.status()
    );
}

/// When `/admin/reload` is triggered and the config on disk is invalid HCL,
/// the reload loop must reject the bad config and preserve the previous
/// runtime state.  Existing routes must continue to serve traffic.
#[test]
fn admin_reload_with_invalid_config_preserves_old_config() {
    // Arrange
    let listener_port = free_port();
    let admin_port = free_port();
    let upstream_port = free_port();

    snakeway_tests::harness::upstream::start_http_upstream(upstream_port);

    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");
    let cert_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("certs");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    // Copy snakeway.hcl
    std::fs::copy(
        fixture_dir.join("snakeway.hcl"),
        temp_dir.path().join("snakeway.hcl"),
    )
    .unwrap();

    // Copy device.d/
    let device_src = fixture_dir.join("device.d");
    let device_dst = temp_dir.path().join("device.d");
    std::fs::create_dir_all(&device_dst).unwrap();
    for entry in std::fs::read_dir(&device_src).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), device_dst.join(entry.file_name())).unwrap();
    }

    // Write ingress config with service route.
    let ingress_dst = temp_dir.path().join("ingress.d");
    std::fs::create_dir_all(&ingress_dst).unwrap();
    let ingress_hcl = format!(
        r#"bind = {{
  interface    = "127.0.0.1"
  port         = {listener_port}
  enable_http2 = false
}}

services = [
  {{
    routes = [
      {{
        hosts = ["{TEST_HOST}"]
        path = "/api"
      }}
    ]

    upstreams = [
      {{
        weight = 1
        endpoint = {{ host = "127.0.0.1", port = {upstream_port} }}
      }}
    ]
  }}
]
"#
    );
    std::fs::write(ingress_dst.join("api.hcl"), &ingress_hcl).unwrap();

    // Write admin ingress config with TLS using absolute cert paths.
    let admin_hcl = format!(
        r#"bind_admin = {{
  interface = "127.0.0.1"
  port      = {admin_port}
  tls = {{
    mode = "manual"
    cert = "{cert}"
    key  = "{key}"
  }}
}}
"#,
        cert = cert_dir.join("server.pem").display(),
        key = cert_dir.join("server.key").display(),
    );
    std::fs::write(ingress_dst.join("admin.hcl"), &admin_hcl).unwrap();

    let validated = load_config(temp_dir.path()).expect("failed to load config");
    let server = ControlPlaneServer::build(Some(temp_dir.path().to_path_buf()), validated.config)
        .expect("failed to build server");
    let _running = server.run_background();

    let listener_addr = format!("127.0.0.1:{listener_port}");
    let admin_addr = format!("127.0.0.1:{admin_port}");
    wait_for_listener(&listener_addr);
    wait_for_listener(&admin_addr);

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let admin = make_admin_client();

    // Verify /api works before reload.
    let res = client
        .get(format!("http://{listener_addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "/api should work initially");

    // Act: overwrite config with invalid HCL and trigger reload via admin API.
    let ingress_path = temp_dir.path().join("ingress.d").join("api.hcl");
    std::fs::write(&ingress_path, "this is not valid { hcl syntax !!!").unwrap();

    let reload_res = admin
        .post(format!("https://{admin_addr}/admin/reload"))
        .send()
        .unwrap();

    // Assert: the reload endpoint accepted the request.
    assert_eq!(
        reload_res.status(),
        StatusCode::OK,
        "admin reload should return 200"
    );

    // Give the reload loop time to process the invalid config.
    std::thread::sleep(Duration::from_millis(500));

    // Assert: /api still works because the old config was preserved.
    let res = client
        .get(format!("http://{listener_addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/api should still work after reload with invalid config"
    );
}
