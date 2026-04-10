use crate::constants::{CERT_SERVER_KEY, CERT_SERVER_PEM, HTTP_UPSTREAM_RESPONSE};

pub fn start_http_upstream(port: u16) {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
        }
    });

    // tiny delay so the listener is actually ready
    thread::sleep(Duration::from_millis(25));
}

/// An upstream that delays before responding. The delay keeps the
/// connection active long enough for concurrent requests to exercise
/// pressure-based load balancing.
pub fn start_slow_http_upstream(port: u16) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
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

    thread::sleep(Duration::from_millis(25));
}

/// An upstream that returns HTTP 500 Internal Server Error for every request.
/// Used to test circuit breaker `count_http_5xx_as_failure` behavior.
pub fn start_http_upstream_that_returns_5xx(port: u16) {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
        }
    });

    thread::sleep(Duration::from_millis(25));
}

/// An upstream that returns a large response body of the given size.
/// Reads request headers before responding to avoid premature close.
pub fn start_http_upstream_with_large_response(port: u16, size_bytes: usize) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
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

    thread::sleep(Duration::from_millis(25));
}

/// An upstream that accepts connections but never responds.
/// Used to test upstream timeout behavior (proxy should return 502/504).
pub fn start_http_upstream_that_hangs(port: u16) {
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let _stream = stream.expect("stream error");
            // Accept the connection but never write a response.
            // The proxy should time out waiting for upstream response.
            thread::sleep(Duration::from_secs(300));
        }
    });

    thread::sleep(Duration::from_millis(25));
}

/// An upstream that reads the full request before responding.
///
/// Unlike `start_http_upstream` (which responds instantly without reading),
/// this variant reads from the socket until EOF or error before writing the
/// response.  This is closer to how real HTTP servers behave and is needed
/// for tests where the proxy must time out waiting for client body bytes —
/// if the upstream responds immediately the proxy forwards that response
/// before it can detect the body underflow.
pub fn start_http_upstream_that_reads_request(port: u16) {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            // Set a read timeout so we don't block forever if the proxy
            // closes the connection (which is the expected outcome when the
            // client body times out).
            let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
            let mut buf = [0u8; 4096];
            // Read until EOF or error — we don't care about the content,
            // we just need to not respond until the proxy is done sending.
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => continue,
                }
            }
            let _ = stream.write_all(HTTP_UPSTREAM_RESPONSE);
        }
    });

    // tiny delay so the listener is actually ready
    thread::sleep(Duration::from_millis(25));
}

/// An upstream that reads request headers and echoes them back as a JSON
/// response body. Used by OTel propagation tests to verify that trace
/// context headers arrive at the upstream.
///
/// Response format: `{"header-name": "value", ...}` (lowercased keys).
pub fn start_http_upstream_that_echoes_headers(port: u16) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");

    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("failed to bind upstream");
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

    thread::sleep(Duration::from_millis(25));
}

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

pub fn start_grpc_upstream(port: u16) {
    use std::thread;
    use tonic::transport::{Identity, Server, ServerTlsConfig};
    use tonic::{Request, Response, Status};

    use helloworld::greeter_server::{Greeter, GreeterServer};
    use helloworld::{HelloReply, HelloRequest};

    #[derive(Default)]
    struct GreeterSvc;

    #[tonic::async_trait]
    impl Greeter for GreeterSvc {
        async fn say_hello(
            &self,
            req: Request<HelloRequest>,
        ) -> Result<Response<HelloReply>, Status> {
            Ok(Response::new(HelloReply {
                message: format!("Hello, {}", req.into_inner().name),
            }))
        }
    }

    thread::spawn(move || {
        let addr = format!("127.0.0.1:{port}").parse().unwrap();

        // Load TLS identity (server cert + key)
        let cert = std::fs::read(CERT_SERVER_PEM).expect("failed to read server.pem");
        let key = std::fs::read(CERT_SERVER_KEY).expect("failed to read server.key");

        let identity = Identity::from_pem(cert, key);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(identity))
                .expect("failed to configure TLS")
                .add_service(GreeterServer::new(GreeterSvc))
                .serve(addr)
                .await
                .expect("gRPC server failed");
        });
    });

    // Give the server a moment to bind + advertise ALPN
    std::thread::sleep(std::time::Duration::from_millis(50));
}

pub fn start_ws_upstream(port: u16) {
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
            let (stream, _) = listener.accept().await.unwrap();

            let mut ws = accept_async(stream).await.unwrap();

            while let Some(msg) = ws.next().await {
                let msg = msg.unwrap();
                ws.send(msg).await.unwrap();
            }
        });
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
}
