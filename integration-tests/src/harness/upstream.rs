use std::io::Write;
use std::net::TcpListener;
use std::sync::Once;
use std::time::Duration;

pub fn start_upstream(port: u16) {
    static STARTED: Once = Once::new();

    STARTED.call_once(|| {
        let addr = format!("127.0.0.1:{port}");
        std::thread::spawn(move || {
            let listener = TcpListener::bind(&addr).expect("failed to bind upstream");

            for stream in listener.incoming() {
                let mut stream = stream.expect("stream error");
                let _ =
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world");
            }
        });

        std::thread::sleep(Duration::from_millis(50));
    });
}
