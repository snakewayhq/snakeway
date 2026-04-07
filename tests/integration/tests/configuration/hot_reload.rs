use integration::constants::TEST_HOST;
use integration::harness::upstream::start_http_upstream;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway_core::testing_api::ControlPlaneServer;
use snakeway_core::testing_api::conf::load_config;
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_listener(addr: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match TcpStream::connect(addr) {
            Ok(_) => return,
            Err(_) => {
                if Instant::now() > deadline {
                    panic!("server failed to start at {addr}");
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

/// Copy the `basic` fixture to a temp dir and patch ports in the HCL.
fn setup_config_dir(listener_port: u16, upstream_port: u16) -> tempfile::TempDir {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("config")
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

    // Copy and patch ingress.d/
    let ingress_src = fixture_dir.join("ingress.d");
    let ingress_dst = temp_dir.path().join("ingress.d");
    std::fs::create_dir_all(&ingress_dst).unwrap();

    let ingress_hcl = std::fs::read_to_string(ingress_src.join("api.hcl")).unwrap();
    let patched = ingress_hcl
        .replace(
            "port         = 8080",
            &format!("port         = {listener_port}"),
        )
        .replace("port = 3001", &format!("port = {upstream_port}"))
        .replace("port = 3002", &format!("port = {upstream_port}"));

    std::fs::write(ingress_dst.join("api.hcl"), &patched).unwrap();

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
    let server = ControlPlaneServer::build(Some(config_dir.path().to_path_buf()), validated.config)
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

    // Modify the ingress config to add a /v2 route.
    let ingress_path = config_dir.path().join("ingress.d").join("api.hcl");
    let ingress_hcl = std::fs::read_to_string(&ingress_path).unwrap();
    let updated = ingress_hcl.replace(
        r#"path = "/api""#,
        &format!(
            "path = \"/api\"\n      }},\n      {{\n        hosts = [\"{TEST_HOST}\"]\n        path = \"/v2\""
        ),
    );
    std::fs::write(&ingress_path, &updated).unwrap();

    // Trigger reload.
    running.reload.notify_reload();

    // Wait for reload to take effect.
    std::thread::sleep(Duration::from_millis(500));

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
