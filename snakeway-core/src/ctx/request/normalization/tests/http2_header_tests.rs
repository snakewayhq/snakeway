use super::test_helpers::input_to_header_map;
use crate::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode, normalize_headers,
};
use crate::ctx::request::normalization::{RejectReason, RewriteReason};
use http::HeaderMap;

//-----------------------------------------------------------------------------
// Accept
//-----------------------------------------------------------------------------
#[test]
fn accept_simple_http2_headers() {
    // Arrange
    let raw = input_to_header_map(&[("host", "example.com"), ("user-agent", "snakeway")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    match result {
        NormalizationOutcome::Accept(h) => {
            assert_eq!(h.as_map().get("host").unwrap(), "example.com");
        }
        _ => panic!("expected Accept"),
    }
}

#[test]
fn accept_te_trailers_as_a_special_case() {
    // Arrange
    let raw = input_to_header_map(&[("te", "trailers")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    assert!(matches!(result, NormalizationOutcome::Accept(_)));
}

//-----------------------------------------------------------------------------
// Reject
//-----------------------------------------------------------------------------
#[test]
fn reject_forbidden_hop_by_hop_connection_header() {
    // Arrange
    let raw = input_to_header_map(&[("connection", "keep-alive")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    assert!(matches!(
        result,
        NormalizationOutcome::Reject {
            reason: RejectReason::HopByHopHeader
        }
    ));
}

#[test]
fn reject_te_header_not_trailers() {
    // Arrange
    let raw = input_to_header_map(&[("te", "gzip")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    assert!(matches!(
        result,
        NormalizationOutcome::Reject {
            reason: RejectReason::HopByHopHeader
        }
    ));
}

#[test]
fn reject_transfer_encoding_header() {
    // Arrange
    let raw = input_to_header_map(&[("transfer-encoding", "chunked")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    assert!(matches!(
        result,
        NormalizationOutcome::Reject {
            reason: RejectReason::HopByHopHeader
        }
    ));
}

//-----------------------------------------------------------------------------
// Rewrite
//-----------------------------------------------------------------------------
#[test]
fn rewrite_fold_duplicate_headers() {
    // Arrange
    let mut raw = HeaderMap::new();
    raw.append("x-test", "a".parse().unwrap());
    raw.append("x-test", "b".parse().unwrap());

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    match result {
        NormalizationOutcome::Rewrite { value, reason } => {
            assert_eq!(reason, RewriteReason::HeaderCanonicalization);
            assert_eq!(value.as_map().get("x-test").unwrap(), "a, b");
        }
        _ => panic!("expected Rewrite"),
    }
}

#[test]
fn rewrite_trim_whitespace() {
    // Arrange
    let raw = input_to_header_map(&[("x-test", "  value ")]);

    // Act
    let result = normalize_headers(&raw, &ProtocolNormalizationMode::Http2);

    // Assert
    match result {
        NormalizationOutcome::Rewrite { value, .. } => {
            assert_eq!(value.as_map().get("x-test").unwrap(), "value");
        }
        _ => panic!("expected Rewrite"),
    }
}
