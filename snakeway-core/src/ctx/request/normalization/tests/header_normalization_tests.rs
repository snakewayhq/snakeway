use crate::ctx::request::normalization::headers::normalize_headers;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};
use http::{HeaderMap, HeaderName, HeaderValue};

fn assert_accept_headers(input: &[(&str, &str)], expected: &[(&str, &str)]) {
    // Arrange
    let mut raw = HeaderMap::new();
    for (k, v) in input {
        let name: HeaderName = k.parse().expect("invalid header name");
        let value: HeaderValue = v.parse().expect("invalid header value");
        raw.insert(name, value);
    }

    // Act
    let outcome = normalize_headers(&raw);

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

fn assert_rewrite_headers(
    input: &[(&str, &str)],
    expected: &[(&str, &str)],
    reason: RewriteReason,
) {
    // Arrange
    let mut raw = HeaderMap::new();
    for (k, v) in input {
        let name: HeaderName = k.parse().expect("invalid header name");
        let value: HeaderValue = v.parse().expect("invalid header value");
        raw.insert(name, value);
    }

    // Act
    let outcome = normalize_headers(&raw);

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

fn assert_reject_headers(input: &[(&str, &str)], reason: RejectReason) {
    // Arrange
    let mut raw = HeaderMap::new();
    for (k, v) in input {
        let name: HeaderName = k.parse().expect("invalid header name");
        let value: HeaderValue = v.parse().expect("invalid header value");
        raw.insert(name, value);
    }

    // Act
    let outcome = normalize_headers(&raw);

    // Assert
    match outcome {
        NormalizationOutcome::Reject { reason: r } => {
            assert_eq!(r, reason);
        }
        other => panic!("Expected Reject, got {:?}", other),
    }
}

//-----------------------------------------------------------------------------
// Accept cases
//-----------------------------------------------------------------------------
#[test]
fn accept_simple_headers() {
    assert_accept_headers(
        &[("host", "example.com"), ("user-agent", "curl/8.0")],
        &[("host", "example.com"), ("user-agent", "curl/8.0")],
    );
}

#[test]
fn accept_header_name_case_insensitive() {
    assert_accept_headers(
        &[("Host", "example.com"), ("USER-AGENT", "curl")],
        &[("host", "example.com"), ("user-agent", "curl")],
    );
}

#[test]
fn accept_multiple_distinct_headers() {
    assert_accept_headers(
        &[("accept", "text/plain"), ("accept-encoding", "gzip")],
        &[("accept", "text/plain"), ("accept-encoding", "gzip")],
    );
}

//-----------------------------------------------------------------------------
// Rewrite cases
//-----------------------------------------------------------------------------
#[test]
fn rewrite_header_name_casing() {
    assert_rewrite_headers(
        &[("Host", "example.com")],
        &[("host", "example.com")],
        RewriteReason::HeaderCanonicalization,
    );
}

#[test]
fn rewrite_fold_duplicate_headers() {
    assert_rewrite_headers(
        &[("accept", "text/plain"), ("accept", "application/json")],
        &[("accept", "text/plain, application/json")],
        RewriteReason::HeaderCanonicalization,
    );
}

#[test]
fn rewrite_trim_whitespace() {
    assert_rewrite_headers(
        &[("x-test", "  value  ")],
        &[("x-test", "value")],
        RewriteReason::HeaderCanonicalization,
    );
}

//-----------------------------------------------------------------------------
// Reject cases
//-----------------------------------------------------------------------------
#[test]
fn reject_invalid_header_name() {
    assert_reject_headers(
        &[("bad header", "oops")],
        RejectReason::HeaderEncodingViolation,
    );
}

#[test]
fn reject_nul_in_header_value() {
    assert_reject_headers(
        &[("x-test", "abc\0def")],
        RejectReason::HeaderEncodingViolation,
    );
}

#[test]
fn reject_hop_by_hop_header() {
    assert_reject_headers(
        &[("connection", "keep-alive")],
        RejectReason::HopByHopHeader,
    );
}
