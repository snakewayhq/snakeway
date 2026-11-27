//! The counting upstream: raw request counting with a quiescence signal.

use super::plain::bind_ephemeral;
use crate::constants::HTTP_UPSTREAM_RESPONSE;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// A counting upstream that parses raw requests on each connection until EOF
/// or a quiet period, counts requests and connections across all
/// connections, and answers every parsed request with the standard canned
/// response, so an acceptance row also observes a forwarded response.
///
/// Parsing is byte tolerant: a request whose head never terminates ends the
/// parse for that connection. The quiet period is a configured bound on how
/// long a connection may sit without new bytes before the parser gives up,
/// because a proxy may hold a keep-alive connection open indefinitely.
///
/// [`CountingUpstream::wait_for_quiescence`] resolves once every connection
/// parser has ended, so tests read counts only after parsing stops.
pub struct CountingUpstream {
    port: u16,
    requests: Arc<AtomicU64>,
    connections: Arc<AtomicU64>,
    quiescence: Arc<Mutex<Receiver<()>>>,
    quiesced: Arc<AtomicU64>,
}

impl CountingUpstream {
    pub fn start(quiet_period: Duration) -> Self {
        let (listener, port) = bind_ephemeral();
        let requests = Arc::new(AtomicU64::new(0));
        let connections = Arc::new(AtomicU64::new(0));
        let (notify_tx, notify_rx) = mpsc::channel();
        let active = Arc::new(Mutex::new(0usize));
        let quiesced = Arc::new(AtomicU64::new(0));

        let requests_for_thread = Arc::clone(&requests);
        let connections_for_thread = Arc::clone(&connections);
        let quiesced_for_thread = Arc::clone(&quiesced);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                connections_for_thread.fetch_add(1, Ordering::SeqCst);
                // The active count rises before the parser thread spawns, so
                // quiescence cannot fire between the accept and the spawn.
                {
                    let mut guard = active.lock().unwrap();
                    *guard += 1;
                }
                quiesced_for_thread.store(0, Ordering::SeqCst);
                let requests = Arc::clone(&requests_for_thread);
                let active = Arc::clone(&active);
                let quiesced = Arc::clone(&quiesced_for_thread);
                let notify_tx = notify_tx.clone();
                thread::spawn(move || {
                    parse_connection(stream, quiet_period, &requests);
                    let left = {
                        let mut guard = active.lock().unwrap();
                        *guard -= 1;
                        *guard
                    };
                    if left == 0 {
                        quiesced.store(1, Ordering::SeqCst);
                        let _ = notify_tx.send(());
                    }
                });
            }
        });

        Self {
            port,
            requests,
            connections,
            quiescence: Arc::new(Mutex::new(notify_rx)),
            quiesced,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn requests(&self) -> u64 {
        self.requests.load(Ordering::SeqCst)
    }

    pub fn connections(&self) -> u64 {
        self.connections.load(Ordering::SeqCst)
    }

    /// Blocks until every connection parser has ended, or the timeout passes.
    /// Returns false on timeout. A call after quiescence returns immediately
    /// unless a new connection started since.
    pub fn wait_for_quiescence(&self, timeout: Duration) -> bool {
        if self.quiesced.load(Ordering::SeqCst) == 1 {
            return true;
        }
        let guard = self.quiescence.lock().unwrap();
        match guard.recv_timeout(timeout) {
            Ok(()) => {
                self.quiesced.store(1, Ordering::SeqCst);
                true
            }
            Err(_) => false,
        }
    }
}

/// Parses requests on one connection until EOF, error, or a quiet period.
/// Each parsed request increments the shared counter and draws the canned
/// response, so a malformed head still counts if its terminator arrived.
fn parse_connection(mut stream: TcpStream, quiet_period: Duration, requests: &AtomicU64) {
    let _ = stream.set_read_timeout(Some(quiet_period));
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let Some(head_end) = wait_for_head(&mut stream, &mut buf) else {
            return;
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        let headers: Vec<(String, String)> = head
            .split("\r\n")
            .skip(1)
            .filter_map(|l| l.split_once(':'))
            .map(|(n, v)| (n.trim().to_lowercase(), v.trim().to_string()))
            .collect();
        let body_len = match header_value(&headers, "transfer-encoding") {
            Some(te) if te.to_ascii_lowercase().contains("chunked") => {
                chunked_total(&mut stream, &mut buf, head_end)
            }
            _ => header_value(&headers, "content-length")
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0),
        };
        fill_until(&mut stream, &mut buf, head_end + body_len);
        if buf.len() < head_end + body_len {
            return;
        }
        requests.fetch_add(1, Ordering::SeqCst);
        if stream.write_all(HTTP_UPSTREAM_RESPONSE).is_err() {
            return;
        }
        let _ = stream.flush();
        buf.drain(..head_end + body_len);
    }
}

fn wait_for_head(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Option<usize> {
    loop {
        if let Some(end) = find_head_end(buf) {
            return Some(end);
        }
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => return None,
        }
    }
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Reads until the buffer holds at least `needed` bytes, or the peer stops.
fn fill_until(stream: &mut TcpStream, buf: &mut Vec<u8>, needed: usize) {
    while buf.len() < needed {
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

/// Consumes one chunked body from `buf[pos..]`, including trailer lines
/// through the terminating empty line, and returns the bytes consumed.
/// Trailer bytes left behind would parse as a phantom next request.
fn chunked_total(stream: &mut TcpStream, buf: &mut Vec<u8>, pos: usize) -> usize {
    let mut cursor = pos;
    loop {
        fill_until(stream, buf, cursor + 2);
        let Some(line_end) = find_crlf(&buf[cursor..]) else {
            return buf.len() - pos;
        };
        let size_line = String::from_utf8_lossy(&buf[cursor..cursor + line_end]).into_owned();
        let size = usize::from_str_radix(size_line.split(';').next().unwrap_or("0").trim(), 16)
            .unwrap_or(0);
        cursor += line_end + 2;
        if size == 0 {
            return consume_trailers(stream, buf, pos, cursor);
        }
        fill_until(stream, buf, cursor + size + 2);
        if buf.len() < cursor + size + 2 {
            return buf.len() - pos;
        }
        cursor += size + 2;
    }
}

/// Consumes trailer lines up to and including the terminating empty line.
/// Returns the end offset minus the body start `pos`.
fn consume_trailers(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    pos: usize,
    mut cursor: usize,
) -> usize {
    loop {
        fill_until(stream, buf, cursor + 2);
        let Some(line_end) = find_crlf(&buf[cursor..]) else {
            return buf.len() - pos;
        };
        if line_end == cursor {
            return cursor + 2 - pos;
        }
        cursor = line_end + 2;
    }
}
