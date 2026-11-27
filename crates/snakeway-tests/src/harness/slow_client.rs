//! A slow client helper built on the replay engine.
//!
//! Staged writes send request bytes in slices with delays, so tests exercise
//! client body and header timeouts without sleeps inside assertions. Abort
//! helpers drop the connection at a chosen point, because the proxy's
//! survival is what the test asserts, not what the client received.

use super::replay::{Exchange, ReadPolicy, WritePlan, replay};

/// The default exchange timeout for helpers that read a response. Tests that
/// assert a timeout pass a shorter bound themselves.
const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Sends `chunks` with `delay` between writes and reads one response.
pub fn send_staged(
    host: &str,
    port: u16,
    chunks: Vec<Vec<u8>>,
    delay: std::time::Duration,
    timeout: std::time::Duration,
) -> Exchange {
    let plan = WritePlan::staged(chunks, delay);
    replay(host, port, &plan, ReadPolicy::OneResponse, timeout)
}

/// Sends the full request in one write, then half closes the write side.
pub fn send_with_half_close(
    host: &str,
    port: u16,
    request: Vec<u8>,
    timeout: std::time::Duration,
) -> Exchange {
    let plan = WritePlan::single(request).with_half_close();
    replay(host, port, &plan, ReadPolicy::OneResponse, timeout)
}

/// Sends `prefix`, waits past `stall`, sends the rest, and reads one
/// response. A stall longer than the proxy's read timeout produces the
/// rejection the batch 2 tests pin.
pub fn send_with_stall(
    host: &str,
    port: u16,
    prefix: Vec<u8>,
    rest: Vec<u8>,
    stall: std::time::Duration,
    timeout: std::time::Duration,
) -> Exchange {
    let plan = WritePlan::staged(vec![prefix, rest], stall);
    replay(host, port, &plan, ReadPolicy::OneResponse, timeout)
}

/// Sends an incomplete request and drops the connection instead of reading a
/// response. The test asserts the proxy's survival and the next request's
/// success afterwards.
pub fn abort_mid_request(host: &str, port: u16, prefix: Vec<u8>) {
    let _ = std::net::TcpStream::connect((host, port)).and_then(|mut stream| {
        use std::io::Write;
        stream.write_all(&prefix)?;
        stream.flush()
    });
}

/// Sends a full request, reads up to `read_bytes` of the response, then drops
/// the connection mid response.
pub fn abort_mid_response(host: &str, port: u16, request: &[u8], read_bytes: usize) {
    use std::io::Read;

    let Ok(mut stream) = std::net::TcpStream::connect((host, port)) else {
        return;
    };
    let _ = stream.set_read_timeout(Some(DEFAULT_TIMEOUT));
    if std::io::Write::write_all(&mut stream, request).is_err() {
        return;
    }
    let mut received = 0usize;
    let mut tmp = [0u8; 512];
    while received < read_bytes {
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => received += n,
        }
    }
    drop(stream);
}
