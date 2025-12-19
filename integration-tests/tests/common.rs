use std::sync::Once;
use std::{io::Write, net::TcpListener, thread};

static UPSTREAM: Once = Once::new();

pub fn spawn_upstream() {
    UPSTREAM.call_once(|| {
        thread::spawn(|| {
            let listener = TcpListener::bind("127.0.0.1:4000").unwrap();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let _ =
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world");
            }
        });
        thread::sleep(std::time::Duration::from_millis(50));
    });
}
