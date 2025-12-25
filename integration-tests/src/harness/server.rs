use crate::harness::config::patch_ports;
use crate::harness::upstream::start_upstream;
use crate::harness::{CapturedEvent, init_test_tracing};
use arc_swap::ArcSwap;
use reqwest::blocking::{Client, RequestBuilder};
use snakeway_core::conf::load_config;
use snakeway_core::server::{build_pingora_server, build_runtime_state};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Handle to a running Snakeway test server.
pub struct TestServer {
    base_url: String,
    client: Client,
}

impl TestServer {
    /// Start a Snakeway instance using a TOML *template* fixture.
    ///
    /// Ports are allocated dynamically and injected into the config.
    ///
    /// This function is fully parallel-safe and nextest-safe.
    pub fn start(fixture: &str) -> Self {
        // Initialize tracing (this must happen first).
        let events = events();
        init_test_tracing(events.clone());
        // Clear events.
        events.lock().unwrap().clear();

        // Allocate a free port for the server.
        let listen_port = free_port();

        // allocate N upstream ports.
        let upstream_ports = [free_port(), free_port(), free_port()];

        // Render config template --> temp file
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("config")
            .join(fixture);

        assert!(
            fixture_dir.exists(),
            "fixture config directory does not exist: {:?}",
            fixture_dir
        );

        for p in upstream_ports {
            start_upstream(p);
        }

        // Load Snakeway config
        let cfg = load_config(&fixture_dir).expect("failed to load fixture config");

        // patch config in memory (no copying, no temp dir)
        let cfg = patch_ports(cfg, listen_port, &upstream_ports);

        // Build initial runtime state (static for tests)
        let runtime_state = build_runtime_state(&cfg).expect("failed to build runtime state");

        // Wrap in ArcSwap (matches production shape)
        let state = Arc::new(ArcSwap::from_pointee(runtime_state));

        // Build server
        let server = build_pingora_server(cfg, state).expect("failed to build snakeway server");

        // Run server in background thread
        thread::spawn(move || {
            server.run_forever();
        });

        let base_url = format!("http://127.0.0.1:{listen_port}");

        // Wait for server to accept connections
        wait_for_server(&base_url);

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("failed to build client");

        Self { base_url, client }
    }

    /// Convenience helper for GET requests.
    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}{}", self.base_url, path))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Poll until the server responds (or panic).
fn wait_for_server(listen_addr: &str) {
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
