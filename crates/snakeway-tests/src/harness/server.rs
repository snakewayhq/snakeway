use crate::constants::{ADMIN_TOKEN, FIXTURES_CONFIG_DIR, TEST_HOST};
use crate::harness::replay_http::replay_http_fixture;
use crate::harness::runtime_patch::patch_runtime;
use crate::harness::upstream::{
    start_grpc_upstream, start_http_upstream, start_http_upstream_that_echoes_headers,
    start_http_upstream_that_echoes_request_line, start_http_upstream_that_reads_request,
    start_ws_upstream,
};
use crate::harness::{CapturedEvent, init_test_tracing};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use snakeway::testing_api::{
    ControlPlaneServer,
    conf::{load_config, types::RuntimeConfig},
    observability::Metrics,
};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

/// Build a reqwest client configured for the admin API (accepts self-signed
/// certs, 5-second timeout, `Authorization: Bearer <ADMIN_TOKEN>` attached
/// by default).
///
/// All admin listeners created via the test `ConfigBuilder` require bearer
/// auth with `ADMIN_TOKEN` or `ADMIN_TOKEN_ALT`. This helper ships the
/// header automatically so callers don't have to.
pub fn admin_client() -> Client {
    admin_client_with_token(ADMIN_TOKEN)
}

/// Build an admin client that sends a specific bearer token. Useful for
/// rotation tests (exercising a second valid token) and negative tests
/// (unknown tokens).
pub fn admin_client_with_token(token: &str) -> Client {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {token}"))
        .expect("token contains invalid header characters");
    headers.insert(AUTHORIZATION, value);
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .default_headers(headers)
        .build()
        .expect("failed to build admin client")
}

/// Build an admin client that sends no `Authorization` header. Used by
/// negative auth tests that expect `401`.
pub fn admin_client_without_auth() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build admin client")
}

/// Handle to a running Snakeway test server.
pub struct TestServer {
    base_urls: Vec<String>,
    listener_addrs: Vec<String>,
    admin_addrs: Vec<String>,
    pub client: Client,
    /// Keeps the control-plane Tokio runtime alive so spawned tasks
    /// (reload loop, ACME reconciliation) are not cancelled.
    _runtime: Option<snakeway::testing_api::RuntimeServer>,
}

impl TestServer {
    pub fn start_grpc_upstream_with_config(cfg: &mut RuntimeConfig) -> Self {
        Self::start_with_config(cfg, start_grpc_upstream)
    }

    pub fn start_ws_upstream_with_config(cfg: &mut RuntimeConfig) -> Self {
        Self::start_with_config(cfg, start_ws_upstream)
    }

    pub fn start_http_upstream_with_config(cfg: &mut RuntimeConfig) -> Self {
        Self::start_with_config(cfg, start_http_upstream)
    }

    pub fn start_http_upstream_that_reads_request_with_config(cfg: &mut RuntimeConfig) -> Self {
        Self::start_with_config(cfg, start_http_upstream_that_reads_request)
    }

    pub fn start_http_upstream_that_echoes_headers_with_config(cfg: &mut RuntimeConfig) -> Self {
        Self::start_with_config(cfg, start_http_upstream_that_echoes_headers)
    }

    pub fn start_http_upstream_that_echoes_request_line_with_config(
        cfg: &mut RuntimeConfig,
    ) -> Self {
        Self::start_with_config(cfg, start_http_upstream_that_echoes_request_line)
    }

    pub fn start_with_config<F>(cfg: &mut RuntimeConfig, start_upstream: F) -> Self
    where
        F: FnMut() -> u16,
    {
        Self::start_with_config_and_metrics(cfg, start_upstream, None)
    }

    pub fn start_with_config_and_metrics<F>(
        cfg: &mut RuntimeConfig,
        mut start_upstream: F,
        metrics: Option<Arc<Metrics>>,
    ) -> Self
    where
        F: FnMut() -> u16,
    {
        // Initialize tracing (this must happen first).
        let events = events();
        init_test_tracing(events.clone());
        // Clear events.
        events.lock().unwrap().clear();

        // Start one upstream per configured tcp upstream. Each starter binds
        // its own listener and reports the bound port, so the port is live
        // before it is patched into the config.
        let upstream_ports = cfg
            .services
            .values()
            .flat_map(|c| c.tcp_upstreams.iter())
            .map(|_| start_upstream())
            .collect::<Vec<_>>();

        // Allocate free ports only for non-redirect listeners.
        let listener_ports = cfg
            .listeners
            .iter()
            .filter(|l| l.redirect.is_none())
            .map(|_| free_port())
            .collect::<Vec<_>>();

        // Patch config in memory.
        patch_runtime(cfg, &listener_ports, &upstream_ports);

        // Build and start via ControlPlaneServer (no config path = no reload).
        let server = match metrics {
            Some(m) => ControlPlaneServer::build_with_metrics(None, cfg.clone(), m, false),
            None => ControlPlaneServer::build(None, cfg.clone(), false),
        }
        .expect("failed to build snakeway server");

        let running = server.run_background();

        Self::from_config(cfg, Some(running))
    }

    fn start_with<F>(fixture: &str, start_upstream: F) -> Self
    where
        F: FnMut() -> u16,
    {
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES_CONFIG_DIR)
            .join(fixture);

        assert!(
            fixture_dir.exists(),
            "fixture config directory does not exist: {:?}",
            fixture_dir
        );

        let mut cfg = load_config(&fixture_dir)
            .expect("failed to load fixture config")
            .config;

        Self::start_with_config(&mut cfg, start_upstream)
    }

    pub fn start_with_ws_upstream(fixture: &str) -> Self {
        Self::start_with(fixture, start_ws_upstream)
    }

    pub fn start_with_grpc_upstream(fixture: &str) -> Self {
        Self::start_with(fixture, start_grpc_upstream)
    }

    pub fn start_with_http_upstream(fixture: &str) -> Self {
        Self::start_with(fixture, start_http_upstream)
    }

    fn from_config(
        cfg: &RuntimeConfig,
        runtime: Option<snakeway::testing_api::RuntimeServer>,
    ) -> Self {
        let listener_addrs: Vec<String> = cfg
            .listeners
            .iter()
            .filter(|l| !l.enable_admin && l.redirect.is_none())
            .map(|l| l.addr.clone())
            .collect();

        let admin_addrs: Vec<String> = cfg
            .listeners
            .iter()
            .filter(|l| l.enable_admin)
            .map(|l| l.addr.clone())
            .collect();

        let base_urls: Vec<String> = listener_addrs
            .iter()
            .map(|a| format!("http://{a}"))
            .collect();

        // Wait for all listeners to accept TCP connections.
        for l in cfg.listeners.iter() {
            wait_for_listener(&l.addr);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("failed to build client");

        Self {
            base_urls,
            listener_addrs,
            admin_addrs,
            client,
            _runtime: runtime,
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        let url = self.base_url().join(path).expect("invalid URL");
        self.client.get(url).header("Host", TEST_HOST)
    }

    pub fn put(&self, path: &str) -> RequestBuilder {
        let url = self.base_url().join(path).expect("invalid URL");
        self.client.put(url).header("Host", TEST_HOST)
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        let url = self.base_url().join(path).expect("invalid URL");
        self.client.post(url).header("Host", TEST_HOST)
    }

    pub fn delete(&self, path: &str) -> RequestBuilder {
        let url = self.base_url().join(path).expect("invalid URL");
        self.client.delete(url).header("Host", TEST_HOST)
    }

    /// Returns the first configured base URL.
    pub fn base_url(&self) -> Url {
        let base_url_str = self.base_urls.first().expect("no base url");
        Url::parse(base_url_str).expect("invalid base URL")
    }

    /// Returns the first configured listener port.
    pub fn port(self) -> u16 {
        self.base_url()
            .port_or_known_default()
            .expect("invalid port")
    }

    /// Returns the first configured listener address (host:port).
    pub fn https_addr(&self) -> &str {
        self.listener_addrs.first().expect("no listener addr")
    }

    /// Returns https://host:port for the first listener.
    pub fn https_url(&self) -> String {
        format!("https://{}", self.https_addr())
    }

    /// Returns https://host:port for the admin API listener.
    pub fn admin_url(&self) -> String {
        format!(
            "https://{}",
            self.admin_addrs.first().expect("no admin listener")
        )
    }

    pub fn replay_http_fixture(self, path: &str) -> String {
        let port = self.port();
        replay_http_fixture(path, port)
    }
}

/// Poll until the TCP port accepts connections (or panic).
/// Accepts either a raw "host:port" address or one with an http(s):// scheme prefix.
pub fn wait_for_listener(listen_addr: &str) {
    let addr = listen_addr
        .strip_prefix("https://")
        .or_else(|| listen_addr.strip_prefix("http://"))
        .unwrap_or(listen_addr);

    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        match TcpStream::connect(addr) {
            Ok(_) => return,
            Err(_) => {
                if Instant::now() > deadline {
                    panic!("server failed to start at {}", listen_addr);
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

static EVENTS: OnceLock<Arc<Mutex<Vec<CapturedEvent>>>> = OnceLock::new();

fn events() -> Arc<Mutex<Vec<CapturedEvent>>> {
    EVENTS
        .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
        .clone()
}

/// Allocate a free port on localhost.
/// This is required to avoid port collisions when running tests in parallel.
pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
