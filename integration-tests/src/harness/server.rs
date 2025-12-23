use crate::harness::config::render_config;
use crate::harness::upstream::start_upstream;
use crate::harness::{CapturedEvent, init_test_tracing};
use reqwest::blocking::{Client, RequestBuilder};
use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::{fs, thread};
use tempfile::TempPath;
use tracing::trace;

/// Global port allocator.
/// We always allocate ports in pairs:
///   - listen port
///   - upstream port
static NEXT_PORT: AtomicU16 = AtomicU16::new(20_000);

fn next_port_pair() -> (u16, u16) {
    let base = NEXT_PORT.fetch_add(2, Ordering::SeqCst);
    (base, base + 1)
}

/// Handle to a running Snakeway test server.
pub struct TestServer {
    base_url: String,
    client: Client,

    // MUST be kept alive or config file is deleted
    #[allow(dead_code)]
    config_file: TempPath,
}

impl TestServer {
    /// Start a Snakeway instance using a TOML *template* fixture.
    ///
    /// Ports are allocated dynamically and injected into the config.
    ///
    /// This function is fully parallel-safe and nextest-safe.
    pub fn start(template: &str) -> Self {
        // Initialize tracing (this must happen first).
        let events = events();
        init_test_tracing(events.clone());
        // Clear events.
        events.lock().unwrap().clear();

        let (listen_port, upstream_port) = next_port_pair();

        // Render config template → temp file
        let config_file = render_config(template, listen_port, upstream_port);
        assert!(config_file.exists(), "rendered config file vanished early");
        trace!(
            path = %config_file.display(),
            contents = %fs::read_to_string(&config_file).unwrap(),
            "rendered snakeway config"
        );
        // Start upstream first
        start_upstream(upstream_port);

        // Load Snakeway config
        let cfg = SnakewayConfig::from_file(config_file.to_str().expect("invalid config path"))
            .expect("failed to load snakeway config");

        // Build server
        let server = build_pingora_server(cfg).expect("failed to build snakeway server");

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

        Self {
            base_url,
            client,
            config_file,
        }
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
