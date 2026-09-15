use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway::testing_api::conf::load_config;
use snakeway::testing_api::{ControlPlaneServer, RuntimeServer};
use snakeway_tests::constants::{ADMIN_TOKEN_FILE, FIXTURES_CONFIG_DIR, TEST_HOST};
use snakeway_tests::harness::server::{admin_client, free_port, wait_for_listener};
use std::path::Path;
use std::time::{Duration, Instant};

/// A running server that loads its configuration from a directory, so a reload reads the files
/// that a test rewrites.
struct ReloadableServer {
    dir: tempfile::TempDir,
    listener_addr: String,
    admin_addr: String,
    listener_port: u16,
    upstream_port: u16,
    _running: RuntimeServer,
}

impl ReloadableServer {
    fn start() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let listener_port = free_port();
        let admin_port = free_port();
        let upstream_port = snakeway_tests::harness::upstream::start_http_upstream();

        copy_fixture_devices(dir.path());
        write_snakeway_hcl(dir.path(), &dir.path().join("first.pid"));
        write_api_ingress(dir.path(), listener_port, upstream_port, "/api");
        write_admin_ingress(dir.path(), admin_port);

        let validated = load_config(dir.path()).expect("failed to load config");
        let server =
            ControlPlaneServer::build(Some(dir.path().to_path_buf()), validated.config, false)
                .expect("failed to build server");
        let running = server.run_background();

        let listener_addr = format!("127.0.0.1:{listener_port}");
        let admin_addr = format!("127.0.0.1:{admin_port}");
        wait_for_listener(&listener_addr);
        wait_for_listener(&admin_addr);

        Self {
            dir,
            listener_addr,
            admin_addr,
            listener_port,
            upstream_port,
            _running: running,
        }
    }

    fn reload(&self) -> StatusCode {
        admin_client()
            .post(format!("https://{}/admin/reload", self.admin_addr))
            .send()
            .expect("reload request failed")
            .status()
    }

    fn status_of(&self, path: &str) -> StatusCode {
        Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("failed to build client")
            .get(format!("http://{}{path}", self.listener_addr))
            .header("Host", TEST_HOST)
            .send()
            .expect("request failed")
            .status()
    }

    /// A reload runs on the control plane after the admin API returns, so poll until the
    /// expected status appears or the deadline passes.
    fn wait_for_status(&self, path: &str, expected: StatusCode) -> StatusCode {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = self.status_of(path);
            if status == expected || Instant::now() >= deadline {
                return status;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn copy_fixture_devices(dir: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic")
        .join("device.d");
    let target = dir.join("device.d");
    std::fs::create_dir_all(&target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
}

fn write_snakeway_hcl(dir: &Path, pid_file: &Path) {
    let hcl = format!(
        r#"server {{
  version  = 1
  pid_file = "{pid_file}"
}}

include {{
  devices   = "device.d/*.hcl"
  ingresses = "ingress.d/*.hcl"
}}
"#,
        pid_file = pid_file.display(),
    );
    std::fs::write(dir.join("snakeway.hcl"), hcl).unwrap();
}

fn write_api_ingress(dir: &Path, listener_port: u16, upstream_port: u16, route_path: &str) {
    let ingress_dir = dir.join("ingress.d");
    std::fs::create_dir_all(&ingress_dir).unwrap();
    let hcl = format!(
        r#"bind = {{
  interface    = "127.0.0.1"
  port         = {listener_port}
  enable_http2 = false
}}

services = [
  {{
    name = "api"
    routes = [
      {{
        hosts = ["{TEST_HOST}"]
        path  = "{route_path}"
      }}
    ]

    upstreams = [
      {{
        weight   = 1
        endpoint = {{ host = "127.0.0.1", port = {upstream_port} }}
      }}
    ]
  }}
]
"#
    );
    std::fs::write(ingress_dir.join("api.hcl"), hcl).unwrap();
}

fn write_admin_ingress(dir: &Path, admin_port: u16) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cert_dir = manifest_dir.join("certs");
    let hcl = format!(
        r#"bind_admin = {{
  interface = "127.0.0.1"
  port      = {admin_port}
  tls = {{
    mode = "manual"
    cert = "{cert}"
    key  = "{key}"
  }}
  auth = {{
    bearer = {{
      token_file = "{token_file}"
    }}
  }}
}}
"#,
        cert = cert_dir.join("server.pem").display(),
        key = cert_dir.join("server.key").display(),
        token_file = manifest_dir.join(ADMIN_TOKEN_FILE).display(),
    );
    std::fs::write(dir.join("ingress.d").join("admin.hcl"), hcl).unwrap();
}

/// A reload that changes only a route applies the new route.
#[test]
fn reload_applies_route_change() {
    // Arrange
    let srv = ReloadableServer::start();
    assert_eq!(srv.status_of("/api"), StatusCode::OK);
    write_api_ingress(srv.dir.path(), srv.listener_port, srv.upstream_port, "/v2");

    // Act
    let reload = srv.reload();

    // Assert
    assert_eq!(reload, StatusCode::OK);
    assert_eq!(srv.wait_for_status("/v2", StatusCode::OK), StatusCode::OK);
    assert_eq!(srv.status_of("/api"), StatusCode::NOT_FOUND);
}

/// A reload that also changes `pid_file` is rejected as a whole, so the route change in the same
/// reload does not apply.
#[test]
fn reload_with_pid_file_change_is_rejected() {
    // Arrange
    let srv = ReloadableServer::start();
    assert_eq!(srv.status_of("/api"), StatusCode::OK);
    write_api_ingress(srv.dir.path(), srv.listener_port, srv.upstream_port, "/v2");
    write_snakeway_hcl(srv.dir.path(), &srv.dir.path().join("second.pid"));

    // Act
    let reload = srv.reload();

    // Assert
    assert_eq!(reload, StatusCode::OK);
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(srv.status_of("/api"), StatusCode::OK);
    assert_eq!(srv.status_of("/v2"), StatusCode::NOT_FOUND);
}
