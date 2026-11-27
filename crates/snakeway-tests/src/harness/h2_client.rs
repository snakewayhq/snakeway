//! A persistent HTTP/2 client handle for integration tests.
//!
//! One TLS connection, many streams. The handle sends requests, streams
//! bodies, reads response heads, bodies, and trailers, resets streams, and
//! reports how the connection ended, which is where a GOAWAY shows up. The
//! per call runtime in `send_h2_request` stays for the tests that use it.

use crate::constants::TEST_HOST;
use crate::h2_over_tls::NoCertVerify;
use bytes::Bytes;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio_rustls::TlsConnector;

/// A live HTTP/2 over TLS connection with its own tokio runtime.
pub struct H2Client {
    rt: Arc<tokio::runtime::Runtime>,
    send: tokio::sync::Mutex<h2::client::SendRequest<Bytes>>,
    end: Arc<Mutex<Option<Result<(), String>>>>,
}

/// One stream on an [`H2Client`] connection.
pub struct H2Stream {
    rt: Arc<tokio::runtime::Runtime>,
    response: Arc<tokio::sync::Mutex<Option<h2::client::ResponseFuture>>>,
    send: Arc<tokio::sync::Mutex<Option<h2::SendStream<Bytes>>>>,
    recv: Arc<tokio::sync::Mutex<Option<h2::RecvStream>>>,
}

/// Why a response head never arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadFailure {
    /// The read timeout passed.
    TimedOut,
    /// The server ended the stream or the connection first.
    Reset(String),
}

impl H2Client {
    /// Connects once. The connection stays open until the server ends it.
    pub fn connect(addr: &str) -> Self {
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime"),
        );
        let (send, end) = rt.block_on(connect_h2(addr));
        Self {
            rt,
            send: tokio::sync::Mutex::new(send),
            end,
        }
    }

    /// Sends a stream. With `end_stream` false the caller sends the body
    /// through the returned handle.
    pub fn request(&self, req: http::Request<()>, end_stream: bool) -> H2Stream {
        let send = self.send.blocking_lock().clone();
        let mut ready = self
            .rt
            .block_on(async { send.ready().await })
            .expect("h2 connection not ready");
        let (response, send_body) = ready
            .send_request(req, end_stream)
            .expect("failed to send request");
        H2Stream {
            rt: Arc::clone(&self.rt),
            response: Arc::new(tokio::sync::Mutex::new(Some(response))),
            send: Arc::new(tokio::sync::Mutex::new(Some(send_body))),
            recv: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// A plain GET stream for the test host root.
    pub fn get(&self, path: &str, authority: &str) -> H2Stream {
        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri(format!("https://{authority}{path}"))
            .body(())
            .expect("valid request");
        self.request(req, true)
    }

    /// How the connection ended. None while it is still running; Some with
    /// an error carries the GOAWAY or protocol error reason as text.
    pub fn connection_end(&self) -> Option<Result<(), String>> {
        self.end.lock().unwrap().clone()
    }
}

impl H2Stream {
    /// Sends body bytes. Waits for flow control capacity, so a stalled
    /// window stalls here and the test observes it through the timeout.
    /// The send stream stays available for [`Self::reset`].
    pub fn send_body(&self, data: &[u8], end_stream: bool) {
        self.rt.block_on(async {
            let mut guard = self.send.lock().await;
            let send = guard.as_mut().expect("send stream already taken");
            send.reserve_capacity(data.len());
            loop {
                if send.capacity() >= data.len() {
                    break;
                }
                let granted = tokio::time::timeout(
                    Duration::from_secs(10),
                    std::future::poll_fn(|cx| send.poll_capacity(cx)),
                )
                .await
                .expect("timed out waiting for flow control capacity");
                if granted.is_none() {
                    panic!("stream closed while waiting for capacity");
                }
            }
            send.send_data(Bytes::copy_from_slice(data), end_stream)
                .expect("failed to send body");
        });
    }

    /// Reads the response head. The error distinguishes a timeout from a
    /// reset or other protocol end.
    pub fn response(
        &self,
        timeout: Duration,
    ) -> Result<(http::StatusCode, http::HeaderMap), HeadFailure> {
        self.rt.block_on(async {
            let mut guard = self.response.lock().await;
            let fut = guard
                .take()
                .ok_or(HeadFailure::Reset("response already read".to_string()))?;
            let resp = match tokio::time::timeout(timeout, fut).await {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => return Err(HeadFailure::Reset(e.to_string())),
                Err(_) => return Err(HeadFailure::TimedOut),
            };
            let (parts, body) = resp.into_parts();
            self.recv.lock().await.replace(body);
            Ok((parts.status, parts.headers))
        })
    }

    /// Reads the response body to its end. Call after [`Self::response`].
    /// The receive stream stays available for [`Self::trailers`].
    pub fn body(&self, timeout: Duration) -> Bytes {
        self.rt.block_on(async {
            let mut guard = self.recv.lock().await;
            let recv = guard.as_mut().expect("response head not read");
            let mut body: Vec<u8> = Vec::new();
            loop {
                match tokio::time::timeout(timeout, recv.data()).await {
                    Ok(Some(Ok(chunk))) => body.extend_from_slice(&chunk),
                    Ok(Some(Err(e))) => panic!("body read error: {e}"),
                    Ok(None) => break,
                    Err(_) => panic!("timed out reading body"),
                }
            }
            Bytes::from(body)
        })
    }

    /// Reads the response trailers. Call after the body is fully read.
    pub fn trailers(&self, timeout: Duration) -> Option<http::HeaderMap> {
        self.rt.block_on(async {
            let mut guard = self.recv.lock().await;
            let recv = guard.as_mut().expect("response head not read");
            match tokio::time::timeout(timeout, recv.trailers()).await {
                Ok(Ok(trailers)) => trailers,
                Ok(Err(e)) => panic!("trailer read error: {e}"),
                Err(_) => panic!("timed out reading trailers"),
            }
        })
    }

    /// Sends RST_STREAM for this stream. Works after the body ended with
    /// end_stream, because the send stream stays retained.
    pub fn reset(&self, reason: h2::Reason) {
        if let Some(send) = self.send.blocking_lock().as_mut() {
            send.send_reset(reason);
        }
    }
}

async fn connect_h2(
    addr: &str,
) -> (
    h2::client::SendRequest<Bytes>,
    Arc<Mutex<Option<Result<(), String>>>>,
) {
    let provider = Arc::new(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("tls protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerify))
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];

    let connector = TlsConnector::from(Arc::new(config));
    let tcp = tokio::net::TcpStream::connect(addr)
        .await
        .expect("tcp connect");
    let server_name = ServerName::try_from(TEST_HOST).expect("valid server name");
    let tls = connector
        .connect(server_name, tcp)
        .await
        .expect("tls handshake");

    let (send, connection) = h2::client::handshake(tls).await.expect("h2 handshake");
    let end = Arc::new(Mutex::new(None));
    let end_for_task = Arc::clone(&end);
    tokio::spawn(async move {
        let result = connection
            .await
            .map_err(|e| format!("connection ended: {e}"));
        *end_for_task.lock().unwrap() = Some(result);
    });
    (send, end)
}
