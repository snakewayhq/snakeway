//! A raw HTTP/2 frame writer for protocol abuse tests.
//!
//! The writer connects over TLS with the h2 ALPN identifier and then ignores
//! the protocol: it never applies or acknowledges the server's SETTINGS, it
//! writes frames the spec forbids, and it reads whatever the server answers.
//! Every frame the builder produces is byte exact, so tests know what left
//! the machine.

use crate::constants::TEST_HOST;
use crate::h2_over_tls::NoCertVerify;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const FRAME_DATA: u8 = 0x0;
pub const FRAME_HEADERS: u8 = 0x1;
pub const FRAME_RST_STREAM: u8 = 0x3;
pub const FRAME_SETTINGS: u8 = 0x4;
pub const FRAME_PING: u8 = 0x6;
pub const FRAME_GOAWAY: u8 = 0x7;
pub const FRAME_WINDOW_UPDATE: u8 = 0x8;

pub const FLAG_END_STREAM: u8 = 0x1;
pub const FLAG_END_HEADERS: u8 = 0x4;
pub const FLAG_ACK: u8 = 0x1;

/// The HTTP/2 connection preface that precedes the first frame.
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// A raw h2 connection. Reads and writes are synchronous with the timeout
/// the exchange budget implies.
pub struct RawH2 {
    stream: rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    deadline: Instant,
}

impl RawH2 {
    /// Connects over TLS with ALPN h2 and sends the connection preface and an
    /// empty SETTINGS frame. Use [`RawH2::connect_with_bad_preface`] for the
    /// preface abuse variant.
    pub fn connect(addr: &str, timeout: Duration) -> Self {
        let mut raw = Self::tls_only(addr, timeout);
        raw.write_all_raw(CONNECTION_PREFACE);
        raw.write_frame(FRAME_SETTINGS, 0, 0, &[]);
        raw
    }

    /// Connects over TLS and writes junk where the preface belongs.
    pub fn connect_with_bad_preface(addr: &str, timeout: Duration) -> Self {
        let mut raw = Self::tls_only(addr, timeout);
        raw.write_all_raw(b"THIS IS NOT A PREFACE\r\n\r\n");
        raw
    }

    fn tls_only(addr: &str, timeout: Duration) -> Self {
        let provider = Arc::new(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("tls protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerify))
            .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec()];

        let connector = rustls::ClientConnection::new(Arc::new(config), server_name())
            .expect("client connection");
        let tcp = TcpStream::connect(addr).expect("tcp connect");
        tcp.set_read_timeout(Some(timeout)).expect("read timeout");
        tcp.set_write_timeout(Some(timeout)).expect("write timeout");
        tcp.set_nodelay(true).expect("nodelay");
        let stream = rustls::StreamOwned::new(connector, tcp);
        Self {
            stream,
            deadline: Instant::now() + timeout,
        }
    }

    /// Writes one frame with a 24 bit length, type, flags, and stream id.
    pub fn write_frame(&mut self, frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) {
        let mut frame = Vec::with_capacity(9 + payload.len());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes()[1..4]);
        frame.push(frame_type);
        frame.push(flags);
        frame.extend_from_slice(&stream_id.to_be_bytes());
        frame.extend_from_slice(payload);
        self.write_all_raw(&frame);
    }

    /// Sends a SETTINGS frame with id and value pairs.
    pub fn send_settings(&mut self, settings: &[(u16, u32)]) {
        let mut payload = Vec::with_capacity(settings.len() * 6);
        for (id, value) in settings {
            payload.extend_from_slice(&id.to_be_bytes());
            payload.extend_from_slice(&value.to_be_bytes());
        }
        self.write_frame(FRAME_SETTINGS, 0, 0, &payload);
    }

    /// Sends a HEADERS frame whose header block is HPACK literals without
    /// indexing and without Huffman coding, one `[name, value]` pair per
    /// header.
    pub fn send_headers(&mut self, stream_id: u32, headers: &[(&str, &str)], end_stream: bool) {
        let mut block = Vec::new();
        for (name, value) in headers {
            block.push(0x00);
            write_hpack_string(&mut block, name.as_bytes());
            write_hpack_string(&mut block, value.as_bytes());
        }
        let flags = if end_stream {
            FLAG_END_STREAM | FLAG_END_HEADERS
        } else {
            FLAG_END_HEADERS
        };
        self.write_frame(FRAME_HEADERS, flags, stream_id, &block);
    }

    /// Sends a DATA frame on a stream.
    pub fn send_data(&mut self, stream_id: u32, data: &[u8], end_stream: bool) {
        let flags = if end_stream { FLAG_END_STREAM } else { 0 };
        self.write_frame(FRAME_DATA, flags, stream_id, data);
    }

    /// Sends RST_STREAM with an error code.
    pub fn send_rst_stream(&mut self, stream_id: u32, error_code: u32) {
        self.write_frame(FRAME_RST_STREAM, 0, stream_id, &error_code.to_be_bytes());
    }

    /// Sends a GOAWAY frame with the last stream id and an error code.
    pub fn send_goaway(&mut self, last_stream_id: u32, error_code: u32) {
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&last_stream_id.to_be_bytes());
        payload.extend_from_slice(&error_code.to_be_bytes());
        self.write_frame(FRAME_GOAWAY, 0, 0, &payload);
    }

    /// Sends `count` empty SETTINGS frames without reading anything between
    /// them.
    pub fn flood_settings(&mut self, count: usize) {
        for _ in 0..count {
            self.write_frame(FRAME_SETTINGS, 0, 0, &[]);
        }
    }

    /// Sends a WINDOW_UPDATE with the increment on a stream.
    pub fn send_window_update(&mut self, stream_id: u32, increment: u32) {
        self.write_frame(FRAME_WINDOW_UPDATE, 0, stream_id, &increment.to_be_bytes());
    }

    /// Reads one frame. None when the peer closes or the deadline passes
    /// first. TLS handshake bytes are drained transparently.
    pub fn read_frame(&mut self) -> Option<(u8, u8, u32, Vec<u8>)> {
        let mut header = [0u8; 9];
        self.read_all_raw(&mut header)?;
        let length = u32::from_be_bytes([0, header[0], header[1], header[2]]) as usize;
        let frame_type = header[3];
        let flags = header[4];
        let stream_id =
            u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fff_ffff;
        let mut payload = vec![0u8; length];
        if length > 0 {
            self.read_all_raw(&mut payload)?;
        }
        Some((frame_type, flags, stream_id, payload))
    }

    /// Returns the exchange deadline instant.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    fn write_all_raw(&mut self, bytes: &[u8]) {
        let mut cursor = 0;
        while cursor < bytes.len() {
            match self.stream.write(&bytes[cursor..]) {
                Ok(0) => panic!("peer closed during write"),
                Ok(n) => cursor += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => panic!("write failed: {e}"),
            }
        }
        self.stream.flush().expect("flush failed");
    }

    fn read_all_raw(&mut self, buf: &mut [u8]) -> Option<()> {
        let mut cursor = 0;
        while cursor < buf.len() {
            match self.stream.read(&mut buf[cursor..]) {
                Ok(0) => return None,
                Ok(n) => cursor += n,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return None;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
        Some(())
    }
}

fn server_name() -> ServerName<'static> {
    ServerName::try_from(TEST_HOST).expect("valid server name")
}

/// Writes an HPACK string literal: no Huffman coding, one varint length.
fn write_hpack_string(out: &mut Vec<u8>, value: &[u8]) {
    write_varint(out, value.len(), 7);
    out.extend_from_slice(value);
}

fn write_varint(out: &mut Vec<u8>, value: usize, prefix_bits: u32) {
    let max = (1usize << prefix_bits) - 1;
    if value < max {
        out.push(value as u8);
        return;
    }
    out.push(max as u8);
    let mut rest = value - max;
    while rest >= 128 {
        out.push((rest % 128 + 128) as u8);
        rest /= 128;
    }
    out.push(rest as u8);
}
