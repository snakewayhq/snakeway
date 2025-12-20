use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;
use std::sync::Once;
use std::{io::Write, net::TcpListener, thread};

use std::path::PathBuf;
use std::time::Duration;

static UPSTREAM: Once = Once::new();

pub fn start_upstream() {
    UPSTREAM.call_once(|| {
        thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:4000").unwrap();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let _ =
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world");
            }
        });
        thread::sleep(Duration::from_millis(50));
    });
}

pub fn load_config(file: &str) -> SnakewayConfig {
    let cfg_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(file);

    SnakewayConfig::from_file(cfg_path.to_str().unwrap())
        .expect("failed to load static.toml config")
}

#[allow(dead_code)]
pub fn start_server(handle: &'static Once, file: &str) {
    handle.call_once(|| {
        let cfg = load_config(file);
        let server = build_pingora_server(cfg).unwrap();

        thread::spawn(move || {
            server.run_forever();
        });

        thread::sleep(Duration::from_millis(200));
    });
}
