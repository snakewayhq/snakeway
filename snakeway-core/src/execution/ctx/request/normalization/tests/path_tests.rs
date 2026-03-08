use crate::ctx::request::NormalizedPath;
use crate::ctx::request::normalization::{
    NormalizationOutcome, RejectReason, RewriteReason, normalize_path,
};
use std::str;

fn assert_accept(path: &str, expected: &str) {
    // Arrange
    let raw = path;

    // Act
    let outcome = normalize_path(raw);

    // Assert
    match outcome {
        NormalizationOutcome::Accept(NormalizedPath(p)) => {
            assert_eq!(p, expected);
        }
        other => panic!("Expected Accept, got {:?}", other),
    }
}

fn assert_rewrite(path: &str, expected: &str) {
    // Arrange
    let raw = path;

    // Act
    let outcome = normalize_path(raw);

    // Assert
    match outcome {
        NormalizationOutcome::Rewrite {
            value: NormalizedPath(p),
            reason: r,
        } => {
            assert_eq!(p, expected);
            assert_eq!(r, RewriteReason::PathCanonicalization);
        }
        other => panic!("Expected Rewrite, got {:?}", other),
    }
}

fn assert_reject(path: &str, reason: RejectReason) {
    // Arrange
    let raw = path;

    // Act
    let outcome = normalize_path(raw);

    // Assert
    match outcome {
        NormalizationOutcome::Reject { reason: r } => {
            assert_eq!(r, reason);
        }
        other => panic!("Expected Reject, got {:?}", other),
    }
}

//-----------------------------------------------------------------------------
// Valid paths (no rewrite)
//-----------------------------------------------------------------------------
#[test]
fn accept_simple_root() {
    let path = "/";

    assert_accept(path, path);
}

#[test]
fn accept_simple_path() {
    let path = "/foo/bar";

    assert_accept(path, path);
}

#[test]
fn accept_numeric_segments() {
    let path = "/v1/api/123";

    assert_accept(path, path);
}

#[test]
fn accept_dash_and_underscore() {
    let path = "/foo-bar_baz";

    assert_accept(path, path);
}

#[test]
fn accept_reserved_characters_encoded() {
    let path = "/foo%2Fbar";

    assert_accept(path, path);
}

//-----------------------------------------------------------------------------
// Path collapse, i.e., // to /
//-----------------------------------------------------------------------------
#[test]
fn rewrite_double_slash() {
    assert_rewrite("//", "/");
}

#[test]
fn rewrite_multiple_slashes() {
    assert_rewrite("/foo///bar", "/foo/bar");
}

#[test]
fn rewrite_trailing_slashes() {
    assert_rewrite("/foo/bar///", "/foo/bar");
}

#[test]
fn accept_root_trailing_slash() {
    assert_accept("/", "/");
}

//-----------------------------------------------------------------------------
// Dot segment removal
//-----------------------------------------------------------------------------
#[test]
fn rewrite_single_dot() {
    assert_rewrite("/./", "/");
}

#[test]
fn rewrite_dot_in_path() {
    assert_rewrite("/foo/./bar", "/foo/bar");
}

#[test]
fn rewrite_double_dot() {
    assert_rewrite("/foo/../bar", "/bar");
}

#[test]
fn rewrite_nested_dot_dot() {
    assert_rewrite("/a/b/c/../../d", "/a/d");
}

//-----------------------------------------------------------------------------
// Path traversal rejection
//-----------------------------------------------------------------------------
#[test]
fn reject_root_escape() {
    assert_reject("/../", RejectReason::PathTraversal);
}

#[test]
fn reject_nested_escape() {
    assert_reject("/a/../../b", RejectReason::PathTraversal);
}

//-----------------------------------------------------------------------------
// Path traversal rejection with percent-encoded traversal (not implemented yet)
//-----------------------------------------------------------------------------
// #[test]
// fn reject_encoded_traversal() {
//     assert_reject("/%2e%2e/", RejectReason::PathTraversal);
// }
//
// #[test]
// fn reject_mixed_encoded_traversal() {
//     assert_reject("/.%2e/", RejectReason::PathTraversal);
// }

//-----------------------------------------------------------------------------
// Percent-decoding (not implemented yet)
//-----------------------------------------------------------------------------
// #[test]
// fn rewrite_percent_decoded_unreserved() {
//     assert_rewrite(
//         "/foo%41bar",
//         "/fooAbar",
//         RewriteReason::PercentDecodeUnreserved,
//     );
// }
//
// #[test]
// fn rewrite_percent_decoded_lowercase_hex() {
//     assert_rewrite("/foo%7e", "/foo~", RewriteReason::PercentDecodeUnreserved);
// }
//
// #[test]
// fn accept_reserved_percent_encoded() {
//     assert_accept("/foo%2Fbar", "/foo%2Fbar");
// }

//-----------------------------------------------------------------------------
// Invalid percent encoding (not implemented yet)
//-----------------------------------------------------------------------------
// #[test]
// fn reject_invalid_percent_encoding_short() {
//     assert_reject("/foo%2", RejectReason::InvalidPercentEncoding);
// }
//
// #[test]
// fn reject_invalid_percent_encoding_non_hex() {
//     assert_reject("/foo%ZZ", RejectReason::InvalidPercentEncoding);
// }
//
// #[test]
// fn reject_percent_at_end() {
//     assert_reject("/foo%", RejectReason::InvalidPercentEncoding);
// }

//-----------------------------------------------------------------------------
// Edge cases
//-----------------------------------------------------------------------------
#[test]
fn rewrite_empty_path_as_root() {
    assert_rewrite("", "/");
}

#[test]
fn rewrite_missing_leading_slash() {
    assert_rewrite("foo/bar", "/foo/bar");
}

#[test]
fn accept_long_path() {
    let long = format!("/{}", "a".repeat(4096));
    assert_accept(&long, &long);
}
