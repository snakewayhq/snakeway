use std::io::Write;
use std::net::TcpListener;
use std::time::Duration;

pub fn start_upstream(port: u16) {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world");
        }
    });

    // tiny delay so the listener is actually ready
    thread::sleep(Duration::from_millis(25));
}
