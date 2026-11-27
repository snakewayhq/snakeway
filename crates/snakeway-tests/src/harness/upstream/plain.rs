//! Plain HTTP upstream starters.

use crate::constants::HTTP_UPSTREAM_RESPONSE;
use std::net::TcpListener;
use std::thread;

pub(super) fn bind_ephemeral() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind upstream");
    let port = listener.local_addr().expect("no local addr").port();
    (listener, port)
}

pub fn start_http_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || serve_http(listener));
    port
}

/// Starts the plain HTTP upstream on a known port, for tests where the proxy
/// config already carries the port and the upstream comes up later (health
/// check recovery, circuit breaker half-open probes). The port sat unbound
/// until this call, so this is the one starter that can still lose a bind
/// race.
pub fn start_http_upstream_on(port: u16) {
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("failed to bind upstream");
    thread::spawn(move || serve_http(listener));
}

fn serve_http(listener: TcpListener) {
    use std::io::Write;

    for stream in listener.incoming() {
        let mut stream = stream.expect("stream error");
        let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
    }
}

/// An upstream that delays before responding. The delay keeps the
/// connection active long enough for concurrent requests to exercise
/// pressure-based load balancing.
pub fn start_slow_http_upstream() -> u16 {
    use std::io::{BufRead, BufReader, Write};
    use std::time::Duration;

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            // Read headers so the proxy considers the request fully sent.
            let reader = BufReader::new(stream.try_clone().unwrap());
            for line in reader.lines() {
                let line = line.unwrap_or_default();
                if line.is_empty() {
                    break;
                }
            }
            // Hold the connection open so concurrent requests see active counts > 0.
            thread::sleep(Duration::from_millis(200));
            let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
        }
    });

    port
}

/// An upstream that returns HTTP 500 Internal Server Error for every request.
/// Used to test circuit breaker `count_http_5xx_as_failure` behavior.
pub fn start_http_upstream_that_returns_5xx() -> u16 {
    use std::io::Write;

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
        }
    });

    port
}

/// An upstream that returns a large response body of the given size.
/// Reads request headers before responding to avoid premature close.
pub fn start_http_upstream_with_large_response(size_bytes: usize) -> u16 {
    use std::io::{BufRead, BufReader, Write};

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        let body = vec![b'X'; size_bytes];
        let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {size_bytes}\r\n\r\n");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let reader = BufReader::new(stream.try_clone().unwrap());
            for line in reader.lines() {
                if line.unwrap_or_default().is_empty() {
                    break;
                }
            }
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
        }
    });

    port
}

/// An upstream that accepts connections but never responds.
/// Used to test upstream timeout behavior (proxy should return 502/504).
pub fn start_http_upstream_that_hangs() -> u16 {
    use std::time::Duration;

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let _stream = stream.expect("stream error");
            // Accept the connection but never write a response.
            // The proxy should time out waiting for upstream response.
            thread::sleep(Duration::from_secs(300));
        }
    });

    port
}

/// An upstream that reads the full request before responding.
///
/// Unlike `start_http_upstream` (which responds instantly without reading),
/// this variant reads from the socket until EOF or error before writing the
/// response.  This is closer to how real HTTP servers behave and is needed
/// for tests where the proxy must time out waiting for client body bytes —
/// if the upstream responds immediately the proxy forwards that response
/// before it can detect the body underflow.
pub fn start_http_upstream_that_reads_request() -> u16 {
    use std::io::{Read, Write};
    use std::time::Duration;

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            // Set a read timeout so we don't block forever if the proxy
            // closes the connection (which is the expected outcome when the
            // client body times out).
            let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
            let mut buf = [0u8; 4096];
            // Read until EOF or error — we don't care about the content,
            // we need to not respond until the proxy is done sending.
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => continue,
                }
            }
            let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
        }
    });

    port
}

/// An upstream that reads request headers and echoes them back as a JSON
/// response body. Used by OTel propagation tests to verify that trace
/// context headers arrive at the upstream.
///
/// Response format: `{"header-name": "value", ...}` (lowercased keys).
pub fn start_http_upstream_that_echoes_headers() -> u16 {
    use std::io::{BufRead, BufReader, Write};

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let reader = BufReader::new(stream.try_clone().unwrap());

            let mut headers = Vec::new();
            for line in reader.lines() {
                let line = line.unwrap_or_default();
                if line.is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    headers.push(format!(
                        "\"{}\":\"{}\"",
                        name.trim().to_lowercase(),
                        value.trim()
                    ));
                }
            }

            let body = format!("{{{}}}", headers.join(","));
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    port
}

/// An upstream that reads the request head and echoes the request line back
/// as a plain-text response body. Used by tests that assert on the method,
/// path, and query the proxy forwarded upstream.
pub fn start_http_upstream_that_echoes_request_line() -> u16 {
    use std::io::{BufRead, BufReader, Write};

    let (listener, port) = bind_ephemeral();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let reader = BufReader::new(stream.try_clone().unwrap());

            let mut lines = reader.lines();
            let request_line = lines.next().and_then(|l| l.ok()).unwrap_or_default();
            for line in lines {
                if line.unwrap_or_default().is_empty() {
                    break;
                }
            }

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                request_line.len(),
                request_line
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    port
}

/// A plain HTTP upstream on a Unix domain socket. The socket is bound before this returns.
pub fn start_unix_http_upstream(path: &std::path::Path) {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;

    let listener = UnixListener::bind(path).expect("failed to bind unix socket upstream");

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
        }
    });
}
