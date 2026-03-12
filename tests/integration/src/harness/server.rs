use crate::constants::{ACME_ORDERS_DIR, TEST_HOST};
use crate::harness::replay_http::replay_http_fixture;
use crate::harness::runtime_patch::patch_runtime;
use crate::harness::upstream::{start_grpc_upstream, start_http_upstream, start_ws_upstream};
use crate::harness::{CapturedEvent, init_test_tracing};
use arc_swap::ArcSwap;
use reqwest::blocking::{Client, RequestBuilder};

use snakeway_core::control_plane::ReloadHandle;
use snakeway_core::control_plane::acme::FilesystemOrderStore;
use snakeway_core::data_plane::build_pingora_server;
use snakeway_core::execution::traffic::TrafficSnapshot;
use snakeway_core::runtime::build_runtime_state;
use snakeway_core::testing_api::conf::load_config;
use snakeway_core::testing_api::conf::types::RuntimeConfig;
use snakeway_core::testing_api::control_plane::acme::{CertManager, MemoryCertStore};
use snakeway_core::testing_api::data_plane::ws_connection_management::WsConnectionManager;
use snakeway_core::testing_api::execution::traffic::TrafficManager;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

/// Handle to a running Snakeway test server.
pub struct TestServer {
    base_urls: Vec<String>,
    listener_addrs: Vec<String>,
    admin_addrs: Vec<String>,
    pub client: Client,
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

    pub fn start_with_config<F>(cfg: &mut RuntimeConfig, start_upstream: F) -> Self
    where
        F: Fn(u16),
    {
        // Initialize tracing (this must happen first).
        let events = events();
        init_test_tracing(events.clone());
        // Clear events.
        events.lock().unwrap().clear();

        //---------------------------------------------------------------------
        // Setup upstreams and listeners, then patch config in-memory.
        //---------------------------------------------------------------------
        // Allocate free port(s) for the upstreams(s).
        let upstream_ports = cfg
            .services
            .iter()
            .flat_map(|(_, c)| c.tcp_upstreams.iter())
            .map(|_| free_port())
            .collect::<Vec<_>>();

        // Start upstream services in background threads.
        for p in upstream_ports.clone() {
            start_upstream(p);
        }

        // Allocate free ports only for non-redirect listeners.
        // Redirect listeners (e.g. the ACME HTTP-01 challenge listener) use fixed ports
        // that match external tooling configuration (see pebble.json httpPort = 5002).
        let listener_ports = cfg
            .listeners
            .iter()
            .filter(|l| l.redirect.is_none())
            .map(|_| free_port())
            .collect::<Vec<_>>();

        // Patch config in memory.
        // This is a bit of magic that ensures all the integration tests can be run in parallel.
        patch_runtime(cfg, &listener_ports, &upstream_ports);

        // Build CertManager when TLS automation is configured.
        let cert_manager: Option<Arc<CertManager>> = {
            let has_tls = cfg.listeners.iter().any(|l| l.tls_termination.is_some());
            if has_tls && let Some(tls_auto) = &cfg.server.tls_automation {
                let order_dir = PathBuf::from(ACME_ORDERS_DIR);
                std::fs::remove_dir_all(&order_dir).expect("failed to remove ACME order store dir");
                std::fs::create_dir_all(&order_dir).expect("failed to create ACME order store dir");
                let cert_store = Arc::new(MemoryCertStore::default());
                let order_store = Arc::new(FilesystemOrderStore::new(order_dir));
                let mgr = Arc::new(CertManager::new(
                    cert_store,
                    order_store,
                    Arc::new(cfg.clone()),
                    tls_auto,
                ));

                // Initialize the ACME client and launch the reconciliation loop in a dedicated
                // thread with its own Tokio runtime.  We cannot call block_on() on the current
                // runtime (the test's multi-thread scheduler), so we spin up a separate one.
                let (tx, rx) = std::sync::mpsc::channel::<()>();
                let cfg_arc = Arc::new(cfg.clone());
                let mgr_thread = mgr.clone();
                thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("failed to build cert-manager runtime");
                    rt.block_on(async {
                        let acme = &cfg_arc.server.tls_automation.as_ref().unwrap().acme;
                        mgr_thread
                            .initialize(acme)
                            .await
                            .expect("cert manager initialization failed");
                        // Signal that initialization is done before entering the reconcile loop.
                        tx.send(()).unwrap();
                        mgr_thread.run_reconciliation().await;
                    });
                });
                rx.recv()
                    .expect("cert manager init thread exited unexpectedly");

                Some(mgr)
            } else {
                None
            }
        };

        let runtime_state =
            build_runtime_state(cfg, &cert_manager).expect("failed to build runtime state");

        // Attach the SNI map so issued ACME certs are served for TLS handshakes.
        if let (Some(manager), Some(tls)) = (cert_manager.as_ref(), runtime_state.tls.as_ref()) {
            manager.attach_tls_sni_map(tls.sni_map.clone());
        }

        let state = Arc::new(ArcSwap::from_pointee(runtime_state));
        let traffic_manager = Arc::new(TrafficManager::new(TrafficSnapshot::from_runtime(
            state.load().as_ref(),
        )));

        // Build server.
        let connection_manager = Arc::new(WsConnectionManager::new());
        let reload = Arc::new(ReloadHandle::new());
        let server = build_pingora_server(
            cfg.clone(),
            state,
            traffic_manager,
            connection_manager,
            cert_manager,
            reload,
        )
        .expect("failed to build snakeway server");

        // Run server in a background thread.
        thread::spawn(move || {
            server.run_forever();
        });

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

        // Wait for all listeners (public, redirect, admin) to accept TCP connections.
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
        }
    }

    fn start_with<F>(fixture: &str, start_upstream: F) -> Self
    where
        F: Fn(u16),
    {
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

    /// Convenience helper for GET requests.
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
        let port = self.port().clone();
        replay_http_fixture(path, port)
    }
}

/// Poll until the TCP port accepts connections (or panic).
/// Accepts either a raw "host:port" address or one with an http(s):// scheme prefix.
fn wait_for_listener(listen_addr: &str) {
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
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
