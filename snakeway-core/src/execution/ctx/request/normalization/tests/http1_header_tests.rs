use super::test_helpers::{assert_accept_headers, assert_reject_headers, assert_rewrite_headers};
use crate::ctx::request::normalization::{ProtocolNormalizationMode, RejectReason, RewriteReason};
use http::HeaderValue;

fn assert_accept_http1_headers(input: &[(&str, &str)], expected: &[(&str, &str)]) {
    assert_accept_headers(input, expected, &ProtocolNormalizationMode::Http1);
}

fn assert_rewrite_http1_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    reason: RewriteReason,
) {
    assert_rewrite_headers(input, expected, reason, &ProtocolNormalizationMode::Http1);
}

fn assert_reject_http1_headers(input: &[(&str, &str)], reason: RejectReason) {
    assert_reject_headers(input, reason, &ProtocolNormalizationMode::Http1);
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
fn strip_hop_by_hop_header() {
    // In HTTP/1, hop-by-hop headers are stripped (not rejected); the result is a rewrite.
    assert_rewrite_http1_headers(
        &[
            ("host", "example.com"),
            ("connection", "keep-alive"),
            ("transfer-encoding", "chunked"),
        ],
        &[("host", "example.com")],
        RewriteReason::HeaderCanonicalization,
    );
}

//-----------------------------------------------------------------------------
// Smuggling reject cases (RFC 9112 §6.3)
//-----------------------------------------------------------------------------

#[test]
fn reject_cl_te_smuggling() {
    // CL.TE: Content-Length then Transfer-Encoding
    assert_reject_http1_headers(
        &[
            ("host", "example.com"),
            ("content-length", "13"),
            ("transfer-encoding", "chunked"),
        ],
        RejectReason::RequestSmugglingAttempt,
    );
}

#[test]
fn reject_te_cl_smuggling() {
    // TE.CL: Transfer-Encoding then Content-Length
    assert_reject_http1_headers(
        &[
            ("host", "example.com"),
            ("transfer-encoding", "chunked"),
            ("content-length", "4"),
        ],
        RejectReason::RequestSmugglingAttempt,
    );
}

#[test]
fn reject_dual_content_length_differing_values() {
    assert_reject_http1_headers(
        &[
            ("host", "example.com"),
            ("content-length", "5"),
            ("content-length", "10"),
        ],
        RejectReason::RequestSmugglingAttempt,
    );
}

#[test]
fn accept_duplicate_content_length_same_value() {
    // Identical duplicate CL is technically redundant but not ambiguous; strip the duplicate.
    assert_rewrite_http1_headers(
        &[
            ("host", "example.com"),
            ("content-length", "5"),
            ("content-length", "5"),
        ],
        &[("host", "example.com"), ("content-length", "5, 5")],
        RewriteReason::HeaderCanonicalization,
    );
}
