use crate::ctx::request::NormalizedPath;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};

/// Normalize a raw URI path into a canonical, safe form.
pub fn normalize_path(raw: &str) -> NormalizationOutcome<NormalizedPath> {
    // Reject NUL bytes outright (never valid in HTTP semantics)
    if raw.as_bytes().contains(&0) {
        return NormalizationOutcome::Reject {
            reason: RejectReason::InvalidUtf8,
        };
    }

    let mut rewritten = false;
    let mut stack: Vec<&str> = Vec::new();

    // Treat empty as root
    let path = if raw.is_empty() { "/" } else { raw };

    // Split on "/" and process segments
    for segment in path.split('/') {
        match segment {
            "" => {
                // collapse repeated slashes
                rewritten = true;
            }
            "." => {
                // remove no-op segment
                rewritten = true;
            }
            ".." => {
                // pop unless this escapes root
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

    // Build the normalized path.
    let mut normalized = String::from("/");
    normalized.push_str(&stack.join("/"));

    // Remove trailing slash except root.
    // This is important, as root is the only time trailing slash is allowed
    // in the canonical representation.
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
