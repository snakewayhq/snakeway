//! The fixture replay entry point the existing test suite uses.
//!
//! The engine lives in [`super::replay`]. This wrapper keeps the old
//! behavior: fixture bytes are normalized, sent in one write, and the response
//! is read until close or timeout.

use super::replay::{ReadPolicy, WritePlan, replay, test_host};
use crate::constants::FIXTURES_HTTP_DIR;
use std::fs;
use std::time::Duration;

/// The exchange timeout the fixture wrapper uses. It matches the read
/// timeout the original replay harness applied.
const FIXTURE_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Normalizes fixture bytes for the wire: LF endings become CRLF, and a
/// header terminator is appended when missing. Raw mode skips this pass.
pub fn normalize_http_newlines(req: Vec<u8>) -> Vec<u8> {
    // convert to string lossily for newline normalization
    let mut s = String::from_utf8_lossy(&req).into_owned();

    // normalize all line endings
    s = s.replace("\r\n", "\n");
    s = s.replace('\n', "\r\n");

    // ensure header terminator
    if !s.ends_with("\r\n\r\n") {
        if s.ends_with("\r\n") {
            s.push_str("\r\n");
        } else {
            s.push_str("\r\n\r\n");
        }
    }

    s.into_bytes()
}

/// Reads an HTTP fixture relative to the fixture directory and replays it
/// against the proxy on `port`. Returns the response bytes as text.
pub fn replay_http_fixture(path: &str, port: u16) -> String {
    let path = format!("{FIXTURES_HTTP_DIR}/{path}");
    let req = fs::read(path).unwrap();
    let req = normalize_http_newlines(req);

    let plan = WritePlan::single(req);
    let exchange = replay(
        test_host(),
        port,
        &plan,
        ReadPolicy::UntilClose,
        FIXTURE_EXCHANGE_TIMEOUT,
    );
    String::from_utf8_lossy(&exchange.raw).to_string()
}
