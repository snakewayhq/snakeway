use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, RequestBuilder};
use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;

use super::tracing::{CapturedEvent, init_test_tracing};
use super::upstream::start_upstream;

pub struct TestServer {
    pub addr: SocketAddr,
    pub client: Client,
    pub events: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl TestServer {
    pub fn start(
        server_once: &'static Once,
        config_file: &str,
        listen_port: u16,
        upstream_port: u16,
    ) -> Self {
        start_upstream(upstream_port);

        let events = Arc::new(Mutex::new(Vec::new()));
        init_test_tracing(events.clone());

        let addr: SocketAddr = format!("127.0.0.1:{listen_port}").parse().unwrap();

        server_once.call_once(|| {
            let cfg = load_config(config_file);
            let server = build_pingora_server(cfg).expect("failed to build server");

            thread::spawn(move || {
                server.run_forever();
            });
        });

        wait_for_server(addr, Duration::from_secs(2));

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        Self {
            addr,
            client,
            events,
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        let path = path.trim_start_matches('/');
        self.client
            .get(format!("http://{}/api/{}", self.addr, path))
    }

    pub fn events(&self) -> std::sync::MutexGuard<'_, Vec<CapturedEvent>> {
        self.events.lock().unwrap()
    }

    pub fn first_identity_json(&self) -> Option<serde_json::Value> {
        let events = self.events();
        let event = events
            .iter()
            .find(|e| e.fields.iter().any(|(k, _)| k == "identity"))?;

        let identity_str = event
            .fields
            .iter()
            .find(|(k, _)| k == "identity")?
            .1
            .clone();

        serde_json::from_str(&identity_str).ok()
    }
}

fn load_config(file: &str) -> SnakewayConfig {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(file);

    SnakewayConfig::from_file(path.to_str().unwrap()).expect("failed to load config")
}

fn wait_for_server(addr: SocketAddr, timeout: Duration) {
    let start = Instant::now();
    loop {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        if start.elapsed() > timeout {
            panic!("server never became ready");
        }
        thread::sleep(Duration::from_millis(50));
    }
}
