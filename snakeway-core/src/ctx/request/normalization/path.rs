use crate::ctx::request::NormalizedPath;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};

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
pub fn normalize_path(path: &str) -> NormalizationOutcome<NormalizedPath> {
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
