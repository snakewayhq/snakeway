//! An HTTP/2 over TLS upstream on the h2 crate server side.
//!
//! Behavior is selected per request path:
//! `/echo` mirrors the request headers into the response body,
//! `/trailers` responds with a body and trailer fields,
//! `/fail` resets the stream without a response,
//! `/hold` parks the stream until released.
//!
//! Every stream records its request body byte count and observed resets, so
//! tests measure what the proxy actually sent and cancelled.

use super::tls::{generate_upstream_identity, test_upstream_tls_acceptor};
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::sync::watch;

/// A running h2 over TLS upstream.
pub struct H2Upstream {
    port: u16,
    body_bytes: Arc<AtomicU64>,
    stream_body_bytes: Arc<Mutex<Vec<u64>>>,
    streams_reset: Arc<AtomicU64>,
    streams_completed: Arc<AtomicU64>,
    /// Release signal for parked `/hold` streams. A release before a stream
    /// parks is still seen, because the watch value stays set.
    release_tx: Arc<watch::Sender<bool>>,
    parked_tx: Arc<watch::Sender<u64>>,
    rt: Arc<tokio::runtime::Runtime>,
}

impl H2Upstream {
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Total request body bytes received across all streams.
    pub fn total_request_body_bytes(&self) -> u64 {
        self.body_bytes.load(Ordering::SeqCst)
    }

    /// Request body byte counts, one entry per stream, in completion order.
    pub fn stream_body_bytes(&self) -> Vec<u64> {
        self.stream_body_bytes.lock().unwrap().clone()
    }

    /// How many client streams the upstream observed as reset.
    pub fn streams_reset(&self) -> u64 {
        self.streams_reset.load(Ordering::SeqCst)
    }

    /// How many streams the upstream answered.
    pub fn streams_completed(&self) -> u64 {
        self.streams_completed.load(Ordering::SeqCst)
    }

    /// Releases streams parked by the `/hold` path, present and future.
    /// `send_replace` sets the value even with no receivers, so a release
    /// before any stream parked is still seen later.
    pub fn release_held(&self) {
        self.release_tx.send_replace(true);
    }

    /// Blocks until `n` streams are parked on the `/hold` path, or the
    /// timeout passes. Returns false on timeout. A test asserting a stall
    /// waits here before it measures anything.
    pub fn wait_until_parked(&self, n: u64, timeout: Duration) -> bool {
        let mut parked_rx = self.parked_tx.subscribe();
        self.rt.block_on(async {
            tokio::time::timeout(timeout, async {
                loop {
                    if *parked_rx.borrow() >= n {
                        return true;
                    }
                    let changed = parked_rx.changed().await;
                    if changed.is_err() {
                        return false;
                    }
                }
            })
            .await
            .unwrap_or(false)
        })
    }
}

/// Starts an h2 over TLS upstream presenting a fresh self-signed certificate.
pub fn start_h2_tls_upstream() -> H2Upstream {
    let (cert_pem, key_pem) = generate_upstream_identity();
    let acceptor = Arc::new(test_upstream_tls_acceptor(&cert_pem, &key_pem));
    let (listener, port) = super::plain::bind_ephemeral();
    listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime"),
    );

    let (release_tx, _) = watch::channel(false);
    let (parked_tx, _) = watch::channel(0u64);
    let release_tx = Arc::new(release_tx);
    let parked_tx = Arc::new(parked_tx);

    let upstream = H2Upstream {
        port,
        body_bytes: Arc::new(AtomicU64::new(0)),
        stream_body_bytes: Arc::new(Mutex::new(Vec::new())),
        streams_reset: Arc::new(AtomicU64::new(0)),
        streams_completed: Arc::new(AtomicU64::new(0)),
        release_tx: Arc::clone(&release_tx),
        parked_tx: Arc::clone(&parked_tx),
        rt: Arc::clone(&rt),
    };

    let body_bytes = Arc::clone(&upstream.body_bytes);
    let stream_body_bytes = Arc::clone(&upstream.stream_body_bytes);
    let streams_reset = Arc::clone(&upstream.streams_reset);
    let streams_completed = Arc::clone(&upstream.streams_completed);

    thread::spawn(move || {
        rt.block_on(async {
            let listener =
                AsyncTcpListener::from_std(listener).expect("failed to adopt h2 listener");
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    continue;
                };
                let acceptor = Arc::clone(&acceptor);
                let body_bytes = Arc::clone(&body_bytes);
                let stream_body_bytes = Arc::clone(&stream_body_bytes);
                let streams_reset = Arc::clone(&streams_reset);
                let streams_completed = Arc::clone(&streams_completed);
                let release_tx = Arc::clone(&release_tx);
                let parked_tx = Arc::clone(&parked_tx);
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    serve_connection(
                        tls,
                        body_bytes,
                        stream_body_bytes,
                        streams_reset,
                        streams_completed,
                        release_tx,
                        parked_tx,
                    )
                    .await;
                });
            }
        });
    });

    upstream
}

#[allow(clippy::too_many_arguments)]
async fn serve_connection<S>(
    tls: S,
    body_bytes: Arc<AtomicU64>,
    stream_body_bytes: Arc<Mutex<Vec<u64>>>,
    streams_reset: Arc<AtomicU64>,
    streams_completed: Arc<AtomicU64>,
    release_tx: Arc<watch::Sender<bool>>,
    parked_tx: Arc<watch::Sender<u64>>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let Ok(mut connection) = h2::server::handshake(tls).await else {
        return;
    };

    while let Some(request) = connection.accept().await {
        let Ok((request, mut respond)) = request else {
            break;
        };
        let body_bytes = Arc::clone(&body_bytes);
        let stream_body_bytes = Arc::clone(&stream_body_bytes);
        let streams_reset = Arc::clone(&streams_reset);
        let streams_completed = Arc::clone(&streams_completed);
        let release_tx = Arc::clone(&release_tx);
        let parked_tx = Arc::clone(&parked_tx);
        tokio::spawn(async move {
            let path = request.uri().path().to_string();
            let headers = request.headers().clone();
            let mut recv = request.into_body();
            let received = read_body(&mut recv).await;
            body_bytes.fetch_add(received.len() as u64, Ordering::SeqCst);

            if path == "/fail" {
                respond.send_reset(h2::Reason::INTERNAL_ERROR);
                return;
            }

            let response = http::Response::builder().status(200).body(()).unwrap();
            let Ok(mut send) = respond.send_response(response, false) else {
                streams_reset.fetch_add(1, Ordering::SeqCst);
                return;
            };

            let final_result = match path.as_str() {
                "/echo" => {
                    let mut echo = Vec::new();
                    for (name, value) in &headers {
                        if let Ok(value) = value.to_str() {
                            echo.extend_from_slice(format!("{name}: {value}\n").as_bytes());
                        }
                    }
                    send.send_data(Bytes::from(echo), true)
                }
                "/trailers" => {
                    let ok = send
                        .send_data(Bytes::from_static(b"payload"), false)
                        .is_ok();
                    if !ok {
                        Err(h2::Error::from(h2::Reason::INTERNAL_ERROR))
                    } else {
                        let mut trailers = http::HeaderMap::new();
                        trailers.insert("x-summary", "done".parse().unwrap());
                        send.send_trailers(trailers)
                    }
                }
                "/hold" => hold_stream(&mut send, release_tx, parked_tx).await,
                _ => send.send_data(Bytes::from_static(b"ok"), true),
            };

            if let Err(e) = final_result {
                if e.is_reset() {
                    streams_reset.fetch_add(1, Ordering::SeqCst);
                }
            } else {
                streams_completed.fetch_add(1, Ordering::SeqCst);
            }
            stream_body_bytes
                .lock()
                .unwrap()
                .push(received.len() as u64);
        });
    }
}

/// Parks the stream until the release signal fires. The watch value carries
/// the permit, so a release before the park is still seen.
async fn hold_stream(
    send: &mut h2::SendStream<Bytes>,
    release_tx: Arc<watch::Sender<bool>>,
    parked_tx: Arc<watch::Sender<u64>>,
) -> Result<(), h2::Error> {
    let mut released_rx = release_tx.subscribe();
    parked_tx.send_modify(|count| *count += 1);
    let result = if *released_rx.borrow() {
        Ok(())
    } else {
        released_rx
            .wait_for(|released| *released)
            .await
            .map(|_| ())
            .map_err(|_| h2::Error::from(h2::Reason::INTERNAL_ERROR))
    };
    parked_tx.send_modify(|count| *count -= 1);
    result?;
    send.send_data(Bytes::from_static(b"released"), true)
}

async fn read_body(recv: &mut h2::RecvStream) -> Vec<u8> {
    let mut body = Vec::new();
    while let Some(chunk) = recv.data().await {
        match chunk {
            Ok(data) => body.extend_from_slice(&data),
            Err(_) => break,
        }
    }
    body
}
