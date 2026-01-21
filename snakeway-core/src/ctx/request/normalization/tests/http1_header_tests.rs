use super::test_helpers::{assert_accept_headers, assert_reject_headers, assert_rewrite_headers};
use crate::ctx::request::normalization::{ProtocolNormalizationMode, RejectReason, RewriteReason};
use http::HeaderValue;

fn assert_accept_http1_headers(input: &[(&str, &str)], expected: &[(&str, &str)]) {
    assert_accept_headers(input, expected, &ProtocolNormalizationMode::Http2);
}

fn assert_rewrite_http1_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    reason: RewriteReason,
) {
    assert_rewrite_headers(input, expected, reason, &ProtocolNormalizationMode::Http2);
}

fn assert_reject_http1_headers(input: &[(&str, &str)], reason: RejectReason) {
    assert_reject_headers(input, reason, &ProtocolNormalizationMode::Http2);
}

//-----------------------------------------------------------------------------
// Accept cases
//-----------------------------------------------------------------------------
#[test]
fn accept_simple_headers() {
    assert_accept_http1_headers(
        &[("host", "example.com"), ("user-agent", "curl/8.0")],
        &[("host", "example.com"), ("user-agent", "curl/8.0")],
    );
}

#[test]
fn accept_header_name_case_insensitive() {
    assert_accept_http1_headers(
        &[("Host", "example.com"), ("USER-AGENT", "curl")],
        &[("host", "example.com"), ("user-agent", "curl")],
    );
}

#[test]
fn accept_multiple_distinct_headers() {
    assert_accept_http1_headers(
        &[("accept", "text/plain"), ("accept-encoding", "gzip")],
        &[("accept", "text/plain"), ("accept-encoding", "gzip")],
    );
}

//-----------------------------------------------------------------------------
// Rewrite cases
//-----------------------------------------------------------------------------
#[test]
fn rewrite_fold_duplicate_headers() {
    assert_rewrite_http1_headers(
        &[("accept", "text/plain"), ("accept", "application/json")],
        &[("accept", "text/plain, application/json")],
        RewriteReason::HeaderCanonicalization,
    );
}

#[test]
fn rewrite_trim_whitespace() {
    assert_rewrite_http1_headers(
        &[("x-test", "  value  ")],
        &[("x-test", "value")],
        RewriteReason::HeaderCanonicalization,
    );
}

//-----------------------------------------------------------------------------
// Reject cases
//-----------------------------------------------------------------------------
#[test]
fn reject_nul_in_header_value_at_parse_time() {
    assert!(HeaderValue::from_bytes(b"abc\0def").is_err());
}

#[test]
fn reject_hop_by_hop_header() {
    assert_reject_http1_headers(
        &[("connection", "keep-alive")],
        RejectReason::HopByHopHeader,
    );
}
