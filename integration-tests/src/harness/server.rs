use crate::harness::config::patch_ports;
use crate::harness::upstream::{start_grpc_upstream, start_http_upstream, start_ws_upstream};
use crate::harness::{CapturedEvent, init_test_tracing};
use arc_swap::ArcSwap;
use reqwest::blocking::{Client, RequestBuilder};
use snakeway_core::conf::load_config;
use snakeway_core::server::{ReloadHandle, build_pingora_server, build_runtime_state};
use snakeway_core::traffic::{TrafficManager, TrafficSnapshot};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Handle to a running Snakeway test server.
pub struct TestServer {
    base_urls: Vec<String>,
    client: Client,
}

impl TestServer {
    fn start_with<F>(fixture: &str, start_upstream: F) -> Self
    where
        F: Fn(u16),
    {
        // Initialize tracing (this must happen first).
        let events = events();
        init_test_tracing(events.clone());
        // Clear events.
        events.lock().unwrap().clear();

        //---------------------------------------------------------------------
        // Gather Configs
        //---------------------------------------------------------------------

        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("config")
            .join(fixture);

        assert!(
            fixture_dir.exists(),
            "fixture config directory does not exist: {:?}",
            fixture_dir
        );

        // Load Snakeway config
        let cfg = load_config(&fixture_dir).expect("failed to load fixture config");

        //---------------------------------------------------------------------
        // Setup upstreams and listeners, then patch config in-memory.
        //---------------------------------------------------------------------

        // Allocate free port(s) for the upstreams(s).
        let upstream_ports = cfg
            .services
            .iter()
            .flat_map(|(_, c)| c.upstream.iter())
            .map(|_| free_port())
            .collect::<Vec<_>>();

        // Start upstream services in background threads.
        for p in upstream_ports.clone() {
            start_upstream(p);
        }

        // Allocate free port(s) for the listener(s).
        let listener_ports = cfg
            .listeners
            .iter()
            .map(|_| free_port())
            .collect::<Vec<_>>();

        // Patch config in memory.
        // This is a bit of magic that ensures all the integration tests can be run in parallel.
        let cfg = patch_ports(cfg, &listener_ports, &upstream_ports);

        // Build the initial runtime state (static for tests).
        let runtime_state = build_runtime_state(&cfg).expect("failed to build runtime state");
        let state = Arc::new(ArcSwap::from_pointee(runtime_state));
        let traffic_manager = Arc::new(TrafficManager::new(TrafficSnapshot::from_runtime(
            state.load().as_ref(),
        )));

        // Build server.
        let reload = ReloadHandle::new();
        let server = build_pingora_server(cfg.clone(), state, traffic_manager, reload)
            .expect("failed to build snakeway server");

        // Run server in a background thread.
        thread::spawn(move || {
            server.run_forever();
        });

        let base_urls = cfg
            .listeners
            .iter()
            .map(|l| format!("http://{}", l.addr.clone()))
            .collect::<Vec<_>>();

        // Wait for listeners(s) to accept connections.
        for base_url in &base_urls {
            wait_for_listener(base_url);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("failed to build client");

        Self { base_urls, client }
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

    /// Convenience helper for GET requests.
    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}{}", self.base_url(), path))
    }

    /// Returns the first configured base URL.
    pub fn base_url(&self) -> &str {
        self.base_urls.first().expect("no base url")
    }
}

/// Poll until the server responds (or panic).
fn wait_for_listener(listen_addr: &str) {
    let addr = listen_addr.strip_prefix("http://").unwrap_or(listen_addr);

    let deadline = Instant::now() + Duration::from_secs(2);

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
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
