use crate::ctx::request::normalization::query::normalize_query;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};
use pretty_assertions::assert_eq;

fn assert_accept_query(raw: &str, expected: &[(&str, &str)]) {
    // Arrange
    let input = raw;

    // Act
    let outcome = normalize_query(input);

    // Assert
    match outcome {
        NormalizationOutcome::Accept(q) => {
            let expected: Vec<(String, String)> = expected
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            assert_eq!(q.pairs(), &expected);
        }
        other => panic!("Expected Accept, got {:?}", other),
    }
}

fn assert_rewrite_query(raw: &str, expected: &[(&str, &str)], reason: RewriteReason) {
    // Arrange
    let input = raw;

    // Act
    let outcome = normalize_query(input);

    // Assert
    match outcome {
        NormalizationOutcome::Rewrite {
            value: q,
            reason: r,
        } => {
            let expected: Vec<(String, String)> = expected
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            assert_eq!(q.pairs(), expected);
            assert_eq!(r, reason);
        }
        other => panic!("Expected Rewrite, got {:?}", other),
    }
}

fn assert_reject_query(raw: &str, reason: RejectReason) {
    // Arrange
    let input = raw;

    // Act
    let outcome = normalize_query(input);

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
fn accept_empty_query() {
    assert_accept_query("", &[]);
}

#[test]
fn accept_single_pair() {
    assert_accept_query("a=1", &[("a", "1")]);
}

#[test]
fn accept_multiple_pairs() {
    assert_accept_query("a=1&b=2", &[("a", "1"), ("b", "2")]);
}

#[test]
fn accept_key_without_value() {
    assert_accept_query("a", &[("a", "")]);
}

#[test]
fn accept_duplicate_keys_preserve_order() {
    assert_accept_query("a=1&a=2", &[("a", "1"), ("a", "2")]);
}

//-----------------------------------------------------------------------------
// Rewrite cases
//-----------------------------------------------------------------------------
#[test]
fn rewrite_query_ordering() {
    assert_rewrite_query(
        "b=2&a=1",
        &[("a", "1"), ("b", "2")],
        RewriteReason::QueryCanonicalization,
    );
}

#[test]
fn rewrite_percent_decode_unreserved() {
    assert_rewrite_query(
        "q=foo%7Ebar",
        &[("q", "foo~bar")],
        RewriteReason::PercentDecodeUnreserved,
    );
}

//-----------------------------------------------------------------------------
// Reject cases
//-----------------------------------------------------------------------------
#[test]
fn reject_invalid_percent_encoding() {
    assert_reject_query("a=%ZZ", RejectReason::InvalidPercentEncoding);
}

#[test]
fn reject_truncated_percent_encoding() {
    assert_reject_query("a=%", RejectReason::InvalidPercentEncoding);
}

#[test]
fn reject_nul_in_query() {
    assert_reject_query("a=1\0", RejectReason::InvalidQueryEncoding);
}
