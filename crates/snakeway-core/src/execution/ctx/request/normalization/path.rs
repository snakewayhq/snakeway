use crate::execution::ctx::request::NormalizedPath;
use crate::execution::ctx::request::normalization::{
    NormalizationOutcome, RejectReason, RewriteReason,
};

/// Normalizes an HTTP request path according to RFC 3986 (URI Generic Syntax) and RFC 9110 (HTTP Semantics).
///
/// This function enforces the following RFC-compliant behaviors:
///
/// - **RFC 3986 § 3.3**: Ensures paths begin with "/" (absolute-path format); relative paths are rewritten.
/// - **RFC 3986 § 6.2.2**: Removes dot-segments ("." and "..") to prevent path traversal attacks and canonicalize the path.
/// - **RFC 3986 § 6.2.3**: Collapses consecutive slashes (e.g., "///" → "/") for path normalization.
/// - **RFC 9110 § 4.1**: Rejects paths containing NUL bytes (0x00) as they violate HTTP message syntax.
/// - **RFC 3986 § 3.3**: Removes trailing slashes except for the root path ("/") to ensure consistent routing.
///
/// The function returns:
/// - `Accept`: Path is already normalized and valid.
/// - `Rewrite`: Path was modified to comply with normalization rules (reason provided).
/// - `Reject`: Path contains invalid or dangerous patterns (e.g., traversal above root, NUL bytes).
pub(crate) fn normalize_path(path: &str) -> NormalizationOutcome<NormalizedPath> {
    // Reject NUL bytes outright (never valid in HTTP semantics).
    if path.as_bytes().contains(&0) {
        return NormalizationOutcome::Reject {
            reason: RejectReason::InvalidUtf8,
        };
    }

    // Treat empty as root (and canonicalize to root).
    if path.is_empty() {
        return NormalizationOutcome::Rewrite {
            value: NormalizedPath("/".to_string()),
            reason: RewriteReason::PathCanonicalization,
        };
    }

    // Short-circuit early if already root.
    if path == "/" {
        return NormalizationOutcome::Accept(NormalizedPath("/".to_string()));
    }

    let mut rewritten = false;
    let mut stack: Vec<&str> = Vec::new();

    // Detect missing leading slash (meaning the raw path is relative).
    if !path.starts_with('/') {
        rewritten = true;
    }

    // Detect collapse of multiple leading slashes.
    if path.starts_with("//") {
        // multiple leading slashes will collapse to one.
        rewritten = true;
    }

    // Strip all leading slashes before splitting
    let body = path.trim_start_matches('/');

    for segment in body.split('/') {
        match segment {
            "" => {
                // repeated or trailing slash...
                // Note, an empty body means root ("/") is already canonical.
                if !body.is_empty() {
                    rewritten = true;
                }
            }
            "." => {
                // no-op segment.
                rewritten = true;
            }
            ".." => {
                // prevent traversal above root.
                if stack.pop().is_none() {
                    return NormalizationOutcome::Reject {
                        reason: RejectReason::PathTraversal,
                    };
                }
                rewritten = true;
            }
            _ => {
                stack.push(segment);
            }
        }
    }

    // Rebuild a normalized path.
    let mut normalized = String::from("/");
    normalized.push_str(&stack.join("/"));

    // Remove trailing slash except root.
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
        rewritten = true;
    }

    let path = NormalizedPath(normalized);

    if rewritten {
        NormalizationOutcome::Rewrite {
            value: path,
            reason: RewriteReason::PathCanonicalization,
        }
    } else {
        NormalizationOutcome::Accept(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ctx::request::NormalizedPath;

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
}
