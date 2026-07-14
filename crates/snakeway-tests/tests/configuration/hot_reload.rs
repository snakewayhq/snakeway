use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway_server::testing_api::ControlPlaneServer;
use snakeway_server::testing_api::conf::load_config;
use snakeway_tests::constants::{FIXTURES_CONFIG_DIR, TEST_HOST};
use snakeway_tests::harness::server::{free_port, wait_for_listener};
use snakeway_tests::harness::upstream::{start_http_upstream, start_slow_http_upstream};
use std::path::Path;
use std::time::Duration;

/// Generate an ingress HCL block with the given ports and route paths.
fn make_ingress_hcl(listener_port: u16, upstream_port: u16, paths: &[&str]) -> String {
    let routes: Vec<String> = paths
        .iter()
        .map(|p| {
            format!(
                r#"      {{
        hosts = ["{TEST_HOST}"]
        path = "{p}"
      }}"#
            )
        })
        .collect();

    format!(
        r#"bind = {{
  interface    = "127.0.0.1"
  port         = {listener_port}
  enable_http2 = false
}}

services = [
  {{
    routes = [
{routes}
    ]

    upstreams = [
      {{
        weight = 1
        endpoint = {{ host = "127.0.0.1", port = {upstream_port} }}
      }}
    ]
  }}
]
"#,
        routes = routes.join(",\n")
    )
}

/// Create a temp config dir with the `basic` fixture's snakeway.hcl and
/// device.d, plus a generated ingress.d/api.hcl with the given ports.
fn setup_config_dir(listener_port: u16, upstream_port: u16) -> tempfile::TempDir {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");

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

    // Write generated ingress config.
    let ingress_dst = temp_dir.path().join("ingress.d");
    std::fs::create_dir_all(&ingress_dst).unwrap();
    let ingress_hcl = make_ingress_hcl(listener_port, upstream_port, &["/api"]);
    std::fs::write(ingress_dst.join("api.hcl"), &ingress_hcl).unwrap();

    temp_dir
}

/// Like `setup_config_dir` but with custom initial routes.
fn setup_config_dir_with_routes(
    listener_port: u16,
    upstream_port: u16,
    paths: &[&str],
) -> tempfile::TempDir {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    std::fs::copy(
        fixture_dir.join("snakeway.hcl"),
        temp_dir.path().join("snakeway.hcl"),
    )
    .unwrap();

    let device_src = fixture_dir.join("device.d");
    let device_dst = temp_dir.path().join("device.d");
    std::fs::create_dir_all(&device_dst).unwrap();
    for entry in std::fs::read_dir(&device_src).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), device_dst.join(entry.file_name())).unwrap();
    }

    let ingress_dst = temp_dir.path().join("ingress.d");
    std::fs::create_dir_all(&ingress_dst).unwrap();
    let ingress_hcl = make_ingress_hcl(listener_port, upstream_port, paths);
    std::fs::write(ingress_dst.join("api.hcl"), &ingress_hcl).unwrap();

    temp_dir
}

/// After triggering a hot-reload that adds a new route, requests to the
/// new path must be served. This verifies that the reload loop re-reads
/// config from disk and swaps the runtime state atomically.
#[test]
fn hot_reload_adds_new_route() {
    let listener_port = free_port();
    let upstream_port = free_port();

    // Start upstream.
    start_http_upstream(upstream_port);

    // Write initial config to temp dir.
    let config_dir = setup_config_dir(listener_port, upstream_port);

    // Load and build with config_path so the reload loop starts.
    let validated = load_config(config_dir.path()).expect("failed to load config");
    let server = ControlPlaneServer::build(
        Some(config_dir.path().to_path_buf()),
        validated.config,
        false,
    )
    .expect("failed to build server");
    let running = server.run_background();

    let addr = format!("127.0.0.1:{listener_port}");
    wait_for_listener(&addr);

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    // Verify initial route works.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "initial /api route should work"
    );

    // Verify /v2 does NOT exist yet.
    let res = client
        .get(format!("http://{addr}/v2"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "/v2 should not exist before reload"
    );

    // Write a new ingress config that adds a /v2 route alongside /api.
    let ingress_path = config_dir.path().join("ingress.d").join("api.hcl");
    let updated = make_ingress_hcl(listener_port, upstream_port, &["/api", "/v2"]);
    std::fs::write(&ingress_path, &updated).unwrap();

    // Trigger reload and poll until the new route is live.
    running.reload.notify_reload();

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let res = client
            .get(format!("http://{addr}/v2"))
            .header("Host", TEST_HOST)
            .send()
            .unwrap();
        if res.status() == StatusCode::OK {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "/v2 route did not become available within 5 seconds after reload"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    // Verify /v2 now works.
    let res = client
        .get(format!("http://{addr}/v2"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/v2 route should work after hot-reload"
    );

    // Verify /api still works.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/api route should still work after hot-reload"
    );
}

/// When a reload is triggered with invalid HCL on disk, the old config
/// must remain active. The reload loop catches the parse error and does
/// not swap runtime state.
#[test]
fn hot_reload_with_invalid_config_preserves_old_routes() {
    // Arrange
    let listener_port = free_port();
    let upstream_port = free_port();

    start_http_upstream(upstream_port);

    let config_dir = setup_config_dir(listener_port, upstream_port);
    let validated = load_config(config_dir.path()).expect("failed to load config");
    let server = ControlPlaneServer::build(
        Some(config_dir.path().to_path_buf()),
        validated.config,
        false,
    )
    .expect("failed to build server");
    let running = server.run_background();

    let addr = format!("127.0.0.1:{listener_port}");
    wait_for_listener(&addr);

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    // Verify /api works before reload.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "/api should work initially");

    // Act: overwrite config with invalid HCL and trigger reload.
    let ingress_path = config_dir.path().join("ingress.d").join("api.hcl");
    std::fs::write(&ingress_path, "this is not valid { hcl syntax !!!").unwrap();
    running.reload.notify_reload();

    // Give the reload loop time to process the invalid config.
    std::thread::sleep(Duration::from_millis(500));

    // Assert: /api must still work because the old config was preserved.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/api should still work after invalid config reload"
    );
}

/// When a reload removes a route, the removed path must return 404
/// while the remaining routes continue to work.
#[test]
fn hot_reload_removes_route() {
    // Arrange: start with both /api and /v2.
    let listener_port = free_port();
    let upstream_port = free_port();

    start_http_upstream(upstream_port);

    let config_dir = setup_config_dir_with_routes(listener_port, upstream_port, &["/api", "/v2"]);
    let validated = load_config(config_dir.path()).expect("failed to load config");
    let server = ControlPlaneServer::build(
        Some(config_dir.path().to_path_buf()),
        validated.config,
        false,
    )
    .expect("failed to build server");
    let running = server.run_background();

    let addr = format!("127.0.0.1:{listener_port}");
    wait_for_listener(&addr);

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    // Verify both routes work.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "/api should work initially");

    let res = client
        .get(format!("http://{addr}/v2"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "/v2 should work initially");

    // Act: write config with only /api and trigger reload.
    let ingress_path = config_dir.path().join("ingress.d").join("api.hcl");
    let updated = make_ingress_hcl(listener_port, upstream_port, &["/api"]);
    std::fs::write(&ingress_path, &updated).unwrap();
    running.reload.notify_reload();

    // Poll until /v2 returns 404.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let res = client
            .get(format!("http://{addr}/v2"))
            .header("Host", TEST_HOST)
            .send()
            .unwrap();
        if res.status() == StatusCode::NOT_FOUND {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "/v2 route did not become 404 within 5 seconds after reload"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    // Assert: /api still works.
    let res = client
        .get(format!("http://{addr}/api"))
        .header("Host", TEST_HOST)
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "/api should still work after removing /v2"
    );
}

/// In-flight requests must complete successfully even when a reload is
/// triggered mid-flight. The slow upstream (200ms delay) ensures
/// requests are still being processed when the reload happens.
#[test]
fn hot_reload_while_requests_are_in_flight() {
    // Arrange
    let listener_port = free_port();
    let upstream_port = free_port();

    start_slow_http_upstream(upstream_port);

    let config_dir = setup_config_dir(listener_port, upstream_port);
    let validated = load_config(config_dir.path()).expect("failed to load config");
    let server = ControlPlaneServer::build(
        Some(config_dir.path().to_path_buf()),
        validated.config,
        false,
    )
    .expect("failed to build server");
    let running = server.run_background();

    let addr = format!("127.0.0.1:{listener_port}");
    wait_for_listener(&addr);

    // Act: send concurrent requests and trigger reload mid-flight.
    let results: Vec<StatusCode> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let addr = &addr;
                s.spawn(move || {
                    // Stagger request starts slightly so some overlap with the reload.
                    if i == 5 {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    let client = Client::builder()
                        .timeout(Duration::from_secs(5))
                        .build()
                        .unwrap();
                    client
                        .get(format!("http://{addr}/api"))
                        .header("Host", TEST_HOST)
                        .send()
                        .unwrap()
                        .status()
                })
            })
            .collect();

        // Trigger reload after a brief delay so some requests are in-flight.
        std::thread::sleep(Duration::from_millis(50));
        let ingress_path = config_dir.path().join("ingress.d").join("api.hcl");
        let updated = make_ingress_hcl(listener_port, upstream_port, &["/api", "/v2"]);
        std::fs::write(&ingress_path, &updated).unwrap();
        running.reload.notify_reload();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Assert: all in-flight requests completed without 5xx errors.
    for (i, status) in results.iter().enumerate() {
        assert_eq!(
            *status,
            StatusCode::OK,
            "request {i} should complete successfully during reload"
        );
    }
}
