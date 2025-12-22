use snakeway_core::config::SnakewayConfig;
use snakeway_core::server::build_pingora_server;
use std::path::PathBuf;
use std::sync::Once;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io::Write, net::TcpListener, thread};

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

    SnakewayConfig::from_file(cfg_path.to_str().unwrap()).expect("failed to load config")
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

pub fn capture_logs<F: FnOnce()>(f: F) -> String {
    use tracing_subscriber::fmt;

    let buf = Arc::new(Mutex::new(String::new()));
    let writer = TestWriter(buf.clone());

    let subscriber = fmt::Subscriber::builder()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    f();

    buf.lock().unwrap().clone()
}

#[derive(Clone)]
pub struct TestWriter(pub Arc<Mutex<String>>);

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.0.lock().unwrap();
        let s = std::str::from_utf8(buf).unwrap();
        lock.push_str(s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
