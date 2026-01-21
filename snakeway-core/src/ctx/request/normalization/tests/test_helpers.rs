use crate::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode, RejectReason, RewriteReason, normalize_headers,
};
use http::{HeaderMap, HeaderName, HeaderValue};

pub(crate) fn input_to_header_map(input: &[(&str, &str)]) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (k, v) in input {
        let name: HeaderName = k.parse().expect("invalid header name");
        let value: HeaderValue = v.parse().expect("invalid header value");
        header_map.append(name, value);
    }
    header_map
}

pub(crate) fn assert_accept_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    protocol_mode: &ProtocolNormalizationMode,
) {
    // Arrange
    let raw = input_to_header_map(input);

    // Act
    let outcome = normalize_headers(&raw, protocol_mode);

    // Assert
    match outcome {
        NormalizationOutcome::Accept(h) => {
            let out = h.as_map();
            assert_eq!(out.len(), expected.len());
            for (k, v) in expected {
                assert_eq!(out.get(*k).unwrap(), v);
            }
        }
        other => panic!("Expected Accept, got {:?}", other),
    }
}

pub(crate) fn assert_rewrite_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    reason: RewriteReason,
    protocol_mode: &ProtocolNormalizationMode,
) {
    // Arrange
    let raw = input_to_header_map(input);

    // Act
    let outcome = normalize_headers(&raw, protocol_mode);

    // Assert
    match outcome {
        NormalizationOutcome::Rewrite {
            value: h,
            reason: r,
        } => {
            let out = h.as_map();
            assert_eq!(out.len(), expected.len());
            for (k, v) in expected {
                assert_eq!(out.get(*k).unwrap(), v);
            }
            assert_eq!(r, reason);
        }
        other => panic!("Expected Rewrite, got {:?}", other),
    }
}

pub(crate) fn assert_reject_headers(
    input: &[(&str, &str)],
    reason: RejectReason,
    protocol_mode: &ProtocolNormalizationMode,
) {
    // Arrange
    let raw = input_to_header_map(input);

    // Act
    let outcome = normalize_headers(&raw, protocol_mode);

    // Assert
    match outcome {
        NormalizationOutcome::Reject { reason: r } => {
            assert_eq!(r, reason);
        }
        other => panic!("Expected Reject, got {:?}", other),
    }
}
