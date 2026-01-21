use crate::ctx::request::NormalizedPath;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};

/// Normalize a raw URI path into a canonical, safe form.
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
