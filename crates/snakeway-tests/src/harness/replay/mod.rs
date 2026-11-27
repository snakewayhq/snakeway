//! A raw byte replay engine for HTTP/1.x exchanges with the proxy.
//!
//! The engine sends fixture bytes exactly as given (raw mode), or a
//! normalized copy of them, under a write plan, and reads the response under a
//! read policy. It returns the raw response bytes and an outcome, so a test
//! can distinguish a rejection status from a close and from a timeout.

mod plan;
mod reader;
mod response;

pub use plan::{Exchange, Outcome, ReadPolicy, WritePlan};
pub use reader::DEFAULT_TRAILING_GRACE;
pub use response::{BodyKind, ResponseHead, parse_chunked, parse_response_head};

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Replays the request carried by `plan` against `addr` under the given
/// policies, with the default trailing grace window. The exchange, including
/// every write, the reads, and the optional trailing drain, must finish
/// inside `timeout`.
pub fn replay(
    host: &str,
    port: u16,
    plan: &WritePlan,
    policy: ReadPolicy,
    timeout: Duration,
) -> Exchange {
    replay_with_grace(host, port, plan, policy, timeout, DEFAULT_TRAILING_GRACE)
}

/// Replays like [`replay`], with the trailing grace window passed in. Tests
/// that hunt a late second response give the window a bound of their own.
pub fn replay_with_grace(
    host: &str,
    port: u16,
    plan: &WritePlan,
    policy: ReadPolicy,
    timeout: Duration,
    trailing_grace: Duration,
) -> Exchange {
    let mut stream = connect(host, port, timeout);
    write_request(&mut stream, plan, timeout);
    reader::read_exchange(&mut stream, plan, policy, timeout, trailing_grace)
}

/// The host the replay engine connects to by default. All test clients come
/// from localhost, so the shared test host constant is the single source.
pub fn test_host() -> &'static str {
    crate::constants::TEST_HOST
}

/// Connects to the proxy, trying each resolved address in turn.
fn connect(host: &str, port: u16, timeout: Duration) -> TcpStream {
    use std::net::ToSocketAddrs;

    let addresses: Vec<std::net::SocketAddr> = (host, port)
        .to_socket_addrs()
        .expect("failed to resolve test host")
        .collect();
    let mut last_error = None;
    for addr in addresses {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                stream.set_nodelay(true).expect("failed to set nodelay");
                return stream;
            }
            Err(e) => last_error = Some(e),
        }
    }
    panic!("failed to connect to proxy: {:?}", last_error);
}

fn write_request(stream: &mut TcpStream, plan: &WritePlan, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    for (index, chunk) in plan.chunks.iter().enumerate() {
        if index > 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            std::thread::sleep(plan.delay.min(remaining));
        }
        stream
            .write_all(chunk)
            .expect("failed to write request chunk");
        stream.flush().expect("failed to flush request chunk");
    }
    if plan.half_close {
        stream
            .shutdown(std::net::Shutdown::Write)
            .expect("failed to half close");
    }
}
