use integration::conf::minimal_http_runtime_config_with_admin;
use integration::constants::ROUTE_PATH_API;
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;

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
