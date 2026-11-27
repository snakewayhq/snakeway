//! Fault upstreams: raw socket behavior for reliability tests.
//!
//! Each starter binds a listener before returning. Variants that react to a
//! test signal return a handle carrying the port and the sender, so the test
//! drives the fault at an exact moment instead of sleeping.

use super::plain::bind_ephemeral;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;

/// The offset of the first CRLFCRLF in the buffer, if one arrived.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Reads the request head (until the empty line), so the proxy considers the
/// request fully sent before the fault applies.
fn read_head(stream: &mut TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            return;
        }
        match stream.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    }
}

/// An upstream that declares a Content-Length body, sends fewer bytes, then
/// closes. The client sees a short body followed by an observed close.
pub fn start_truncated_body_upstream(declared: usize, sent: usize) -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\n\r\n");
            let _ = stream.write_all(header.as_bytes());
            let body = vec![b'X'; sent];
            let _ = stream.write_all(&body);
            let _ = stream.flush();
            drop(stream);
        }
    });
    port
}

/// An upstream that declares a body, sends part of it, then resets the
/// connection instead of closing it.
pub fn start_reset_mid_response_upstream(declared: usize, sent: usize) -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\n\r\n");
            let _ = stream.write_all(header.as_bytes());
            let body = vec![b'X'; sent];
            let _ = stream.write_all(&body);
            let _ = stream.flush();
            force_reset(&stream);
            drop(stream);
        }
    });
    port
}

/// Forces a TCP RST by closing a socket with SO_LINGER zero. A failed
/// setsockopt would turn the fault into a plain close, so the call must
/// succeed.
fn force_reset(stream: &TcpStream) {
    let linger = libc::linger {
        l_onoff: 1,
        l_linger: 0,
    };
    let result = unsafe {
        libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &linger as *const libc::linger as *const libc::c_void,
            std::mem::size_of::<libc::linger>() as libc::socklen_t,
        )
    };
    assert_eq!(result, 0, "failed to set SO_LINGER for a forced reset");
}

/// An upstream that sends bytes that are not HTTP at all.
pub fn start_garbage_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.write_all(b"\x00\x01\x02 NOT HTTP\r\n\r\n");
        }
    });
    port
}

/// An upstream that sends a valid chunked response with trailer fields after
/// the terminal chunk.
pub fn start_chunked_response_with_trailers_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-Summary\r\n\r\n\
                  5\r\nhello\r\n6\r\n world\r\n0\r\nX-Summary: 11 bytes\r\n\r\n",
            );
        }
    });
    port
}

/// An upstream that accepts a connection and sends only part of a response
/// head (no terminator). The head stays incomplete until the test signals, or
/// forever when the sender is dropped.
pub struct IncompleteHeadersUpstream {
    pub port: u16,
    release: Sender<()>,
}

impl IncompleteHeadersUpstream {
    /// Lets the upstream finish the connection (by closing it).
    pub fn release(&self) {
        let _ = self.release.send(());
    }
}

pub fn start_incomplete_headers_upstream() -> IncompleteHeadersUpstream {
    let (listener, port) = bind_ephemeral();
    let (release_tx, release_rx) = channel::<()>();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Len");
            let _ = stream.flush();
            let _ = release_rx.recv();
            drop(stream);
        }
    });
    IncompleteHeadersUpstream {
        port,
        release: release_tx,
    }
}

/// An upstream that sends an interim 100 Continue followed by a final 200.
pub fn start_hundred_then_final_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
            let _ = stream.flush();
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
        }
    });
    port
}

/// An upstream that serves requests normally, then closes the connection the
/// moment the test signals, before parsing a further request. Every
/// connection watches the same flag, so tests with several pooled
/// connections can kill any of them. The connection closes without reading
/// the bytes of a next request, which is what a pooled reuse test needs to
/// observe.
pub struct KillIdleUpstream {
    pub port: u16,
    kill: Arc<AtomicBool>,
}

impl KillIdleUpstream {
    /// Signals every connection to close before its next parse.
    pub fn kill_idle(&self) {
        self.kill.store(true, Ordering::SeqCst);
    }
}

pub fn start_kill_idle_upstream() -> KillIdleUpstream {
    let (listener, port) = bind_ephemeral();
    let kill = Arc::new(AtomicBool::new(false));
    let kill_for_thread = Arc::clone(&kill);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let kill = Arc::clone(&kill_for_thread);
            thread::spawn(move || {
                let mut stream = stream;
                let mut buf: Vec<u8> = Vec::new();
                loop {
                    if wait_for_head_or_kill(&mut stream, &mut buf, &kill).is_none() {
                        return;
                    }
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
                    let _ = stream.flush();
                }
            });
        }
    });
    KillIdleUpstream { port, kill }
}

/// Reads one request head, or returns None when the kill flag rose first.
/// The short read timeout makes the flag check run between every read, so a
/// signal is seen before the next request is parsed.
fn wait_for_head_or_kill(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    kill: &AtomicBool,
) -> Option<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
    loop {
        if kill.load(Ordering::SeqCst) {
            return None;
        }
        if find_head_end(buf).is_some() {
            return Some(());
        }
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => return None,
        }
    }
}

/// An upstream whose response carries both Content-Length and
/// Transfer-Encoding: chunked.
pub fn start_conflicting_cl_te_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n\
                  5\r\nhello\r\n0\r\n\r\n",
            );
        }
    });
    port
}

/// An upstream whose response carries two Content-Length fields.
pub fn start_duplicate_content_length_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello",
            );
        }
    });
    port
}

/// An upstream whose response carries hop-by-hop fields. The response is
/// otherwise well formed, so a test isolates what the proxy does with the
/// hop-by-hop names themselves.
pub fn start_hop_by_hop_response_upstream() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\n\
                  Connection: close\r\n\
                  Keep-Alive: timeout=5\r\n\
                  TE: trailers\r\n\
                  Proxy-Authorization: Basic Zm9v\r\n\
                  Trailer: X-Summary\r\n\
                  Upgrade: h2c\r\n\
                  Proxy-Authenticate: Basic realm=\"up\"\r\n\
                  Transfer-Encoding: chunked\r\n\
                  X-End-To-End: kept\r\n\
                  \r\n\
                  0\r\n\r\n",
            );
        }
    });
    port
}

/// An upstream that declares a body and drips body bytes only when the test
/// sends them, so the test controls the exact send moments. Dropping the
/// handle closes the connection.
pub struct DripBodyUpstream {
    pub port: u16,
    body_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
}

impl DripBodyUpstream {
    /// Writes body bytes to the response now.
    pub fn drip(&self, bytes: &[u8]) {
        let _ = self.body_tx.send(bytes.to_vec());
    }
}

pub fn start_drip_body_upstream(declared: u64) -> DripBodyUpstream {
    let (listener, port) = bind_ephemeral();
    let (body_tx, body_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(16);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_head(&mut stream);
            let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\n\r\n");
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.flush();
            while let Ok(bytes) = body_rx.recv() {
                if stream.write_all(&bytes).is_err() || stream.flush().is_err() {
                    break;
                }
            }
            drop(stream);
        }
    });
    DripBodyUpstream { port, body_tx }
}
