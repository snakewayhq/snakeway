use super::test_helpers::{assert_accept_headers, assert_reject_headers, assert_rewrite_headers};
use crate::ctx::request::normalization::ProtocolNormalizationMode;
use crate::ctx::request::normalization::{RejectReason, RewriteReason};

fn assert_accept_http2_headers(input: &[(&str, &str)], expected: &[(&str, &str)]) {
    assert_accept_headers(input, expected, &ProtocolNormalizationMode::Http2);
}

fn assert_rewrite_http2_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    reason: RewriteReason,
) {
    assert_rewrite_headers(input, expected, reason, &ProtocolNormalizationMode::Http2);
}

fn assert_reject_http2_headers(input: &[(&str, &str)], reason: RejectReason) {
    assert_reject_headers(input, reason, &ProtocolNormalizationMode::Http2);
}

//-----------------------------------------------------------------------------
// Accept
//-----------------------------------------------------------------------------
#[test]
fn accept_simple_http2_headers() {
    assert_accept_http2_headers(
        &[("host", "example.com"), ("user-agent", "snakeway")],
        &[("host", "example.com"), ("user-agent", "snakeway")],
    );
}

#[test]
fn accept_te_trailers_as_a_special_case() {
    assert_accept_http2_headers(&[("te", "trailers")], &[("te", "trailers")]);
}

//-----------------------------------------------------------------------------
// Reject
//-----------------------------------------------------------------------------
#[test]
fn reject_forbidden_hop_by_hop_connection_header() {
    assert_reject_http2_headers(
        &[("connection", "keep-alive")],
        RejectReason::HopByHopHeader,
    );
}

#[test]
fn reject_te_header_not_trailers() {
    assert_reject_http2_headers(&[("te", "gzip")], RejectReason::HopByHopHeader);
}

#[test]
fn reject_transfer_encoding_header() {
    assert_reject_http2_headers(
        &[("transfer-encoding", "chunked")],
        RejectReason::HopByHopHeader,
    );
}

//-----------------------------------------------------------------------------
// Rewrite
//-----------------------------------------------------------------------------
#[test]
fn rewrite_fold_duplicate_headers() {
    // // Arrange
    // let mut raw = HeaderMap::new();
    // raw.append("x-test", "a".parse().unwrap());
    // raw.append("x-test", "b".parse().unwrap());
    //
    // // Act
    // let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);
    //
    // // Assert
    // match result {
    //     NormalizationOutcome::Rewrite { value, reason } => {
    //         assert_eq!(reason, RewriteReason::HeaderCanonicalization);
    //         assert_eq!(value.as_map().get("x-test").unwrap(), "a, b");
    //     }
    //     _ => panic!("expected Rewrite"),
    // }

    assert_rewrite_http2_headers(
        &[("x-test", "a"), ("x-test", "b")],
        &[("x-test", "a, b")],
        RewriteReason::HeaderCanonicalization,
    );
}

#[test]
fn rewrite_trim_whitespace() {
    assert_rewrite_http2_headers(
        &[("x-test", "  value  ")],
        &[("x-test", "value")],
        RewriteReason::HeaderCanonicalization,
    );
}
