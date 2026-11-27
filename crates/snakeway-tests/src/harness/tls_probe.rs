//! A raw legacy TLS probe for the version floor test.
//!
//! rustls 0.23 cannot speak TLS 1.0 or 1.1, and an openssl subprocess depends
//! on the host build's security level, so the probe sends a fixed ClientHello
//! by hand and reads the alert the server answers with.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The legacy protocol version the probe offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyVersion {
    Tls10,
    Tls11,
}

impl LegacyVersion {
    /// The ClientHello legacy_version bytes.
    pub fn version_bytes(self) -> [u8; 2] {
        match self {
            LegacyVersion::Tls10 => [0x03, 0x01],
            LegacyVersion::Tls11 => [0x03, 0x02],
        }
    }
}

/// What the server answered a legacy ClientHello with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The server answered with a TLS alert: level and description bytes.
    Alert { level: u8, description: u8 },
    /// The server closed without an alert.
    Closed,
    /// The server kept the connection open past the deadline.
    NoAnswer,
    /// The server completed a handshake. This would fail the version floor.
    HandshakeCompleted,
}

/// Offers a TLS 1.0 or 1.1 ClientHello and reports the server's answer.
pub fn probe_legacy_client_hello(port: u16, version: LegacyVersion) -> ProbeOutcome {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");

    let hello = client_hello(version, b"snakeway.test");
    stream
        .write_all(&hello)
        .expect("failed to write ClientHello");

    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return ProbeOutcome::Closed,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(outcome) = classify(&buf) {
                    return outcome;
                }
            }
            Err(_) => return ProbeOutcome::NoAnswer,
        }
    }
}

/// A complete ClientHello: legacy version TLS 1.0/1.1, cipher suites rustls
/// parses, and the extensions rustls requires before it reaches the version
/// check: SNI, signature algorithms, and supported groups.
fn client_hello(version: LegacyVersion, sni: &[u8]) -> Vec<u8> {
    let [major, minor] = version.version_bytes();

    // server_name extension payload: one list entry of type host_name.
    let mut sni_ext_data = Vec::new();
    sni_ext_data.extend_from_slice(&(sni.len() as u16 + 3).to_be_bytes());
    sni_ext_data.push(0);
    sni_ext_data.extend_from_slice(&(sni.len() as u16).to_be_bytes());
    sni_ext_data.extend_from_slice(sni);

    let mut extensions = Vec::new();
    extensions.extend_from_slice(&0x0000u16.to_be_bytes());
    extensions.extend_from_slice(&(sni_ext_data.len() as u16).to_be_bytes());
    extensions.extend_from_slice(&sni_ext_data);

    // signature_algorithms: the algorithms rustls accepts for RSA keys.
    let sig_algorithms: [u16; 3] = [0x0401, 0x0501, 0x0804];
    let mut sig_algs_data = Vec::new();
    for algorithm in sig_algorithms {
        sig_algs_data.extend_from_slice(&algorithm.to_be_bytes());
    }
    let mut sig_algs = Vec::new();
    sig_algs.extend_from_slice(&(sig_algs_data.len() as u16).to_be_bytes());
    sig_algs.extend_from_slice(&sig_algs_data);
    extensions.extend_from_slice(&0x000du16.to_be_bytes());
    extensions.extend_from_slice(&(sig_algs.len() as u16).to_be_bytes());
    extensions.extend_from_slice(&sig_algs);

    // supported_groups: x25519 and secp256r1.
    let groups: [u16; 2] = [0x001d, 0x0017];
    let mut groups_data = Vec::new();
    for group in groups {
        groups_data.extend_from_slice(&group.to_be_bytes());
    }
    let mut group_list = Vec::new();
    group_list.extend_from_slice(&(groups_data.len() as u16).to_be_bytes());
    group_list.extend_from_slice(&groups_data);
    extensions.extend_from_slice(&0x000au16.to_be_bytes());
    extensions.extend_from_slice(&(group_list.len() as u16).to_be_bytes());
    extensions.extend_from_slice(&group_list);

    let mut body = Vec::new();
    body.extend_from_slice(&[major, minor]);
    body.extend_from_slice(&[0x01; 32]);
    body.push(0x00);
    // Cipher suites: TLS_RSA_WITH_AES_128_CBC_SHA and
    // TLS_RSA_WITH_AES_256_CBC_SHA, both parseable legacy suites.
    body.extend_from_slice(&[0x00, 0x04, 0x00, 0x2f, 0x00, 0x35]);
    body.push(0x01);
    body.push(0x00);
    body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    body.extend_from_slice(&extensions);

    let mut handshake = Vec::with_capacity(body.len() + 4);
    handshake.push(0x01);
    let len = body.len();
    handshake.extend_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8]);
    handshake.extend_from_slice(&body);

    let mut record = Vec::with_capacity(handshake.len() + 5);
    record.push(0x16);
    record.extend_from_slice(&[major, minor]);
    let hlen = handshake.len();
    record.extend_from_slice(&[(hlen >> 8) as u8, hlen as u8]);
    record.extend_from_slice(&handshake);
    record
}

/// Classifies the first server answer from received bytes.
fn classify(buf: &[u8]) -> Option<ProbeOutcome> {
    if buf.len() < 2 {
        return None;
    }
    match buf[0] {
        // Alert record.
        0x15 => {
            if buf.len() < 7 {
                return None;
            }
            Some(ProbeOutcome::Alert {
                level: buf[5],
                description: buf[6],
            })
        }
        // Handshake record: a ServerHello means the handshake continued.
        0x16 => Some(ProbeOutcome::HandshakeCompleted),
        _ => None,
    }
}
