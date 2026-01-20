use crate::ctx::request::NormalizedHeaders;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};
use http::{HeaderMap, HeaderName, HeaderValue};

pub fn normalize_headers(raw: &HeaderMap) -> NormalizationOutcome<NormalizedHeaders> {
    let mut rewritten = false;
    let mut out = HeaderMap::new();

    for (name, value) in raw.iter() {
        let name_str = name.as_str();

        // Reject hop-by-hop headers
        if is_hop_by_hop(name_str) {
            return NormalizationOutcome::Reject {
                reason: RejectReason::HopByHopHeader,
            };
        }

        // Canonicalize header name (lowercase)
        let canonical_name: HeaderName = match name_str.to_ascii_lowercase().parse() {
            Ok(h) => h,
            Err(_) => {
                return NormalizationOutcome::Reject {
                    reason: RejectReason::HeaderEncodingViolation,
                };
            }
        };

        if name_str != canonical_name.as_str() {
            rewritten = true;
        }

        // Decode header value safely
        let value_str = match value.to_str() {
            Ok(v) => v,
            Err(_) => {
                return NormalizationOutcome::Reject {
                    reason: RejectReason::HeaderEncodingViolation,
                };
            }
        };

        // Reject NUL bytes
        if value_str.as_bytes().contains(&0) {
            return NormalizationOutcome::Reject {
                reason: RejectReason::HeaderEncodingViolation,
            };
        }

        // Trim whitespace
        let trimmed = value_str.trim();
        if trimmed != value_str {
            rewritten = true;
        }

        let val = match HeaderValue::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                return NormalizationOutcome::Reject {
                    reason: RejectReason::HeaderEncodingViolation,
                };
            }
        };

        // Fold duplicate headers
        match out.get_mut(&canonical_name) {
            Some(existing) => {
                let merged = match existing.to_str() {
                    Ok(e) => format!("{}, {}", e, trimmed),
                    Err(_) => {
                        return NormalizationOutcome::Reject {
                            reason: RejectReason::HeaderEncodingViolation,
                        };
                    }
                };
                *existing = HeaderValue::from_str(&merged).unwrap();
                rewritten = true;
            }
            None => {
                out.insert(canonical_name, val);
            }
        }
    }

    let normalized = NormalizedHeaders::new(out);

    if rewritten {
        NormalizationOutcome::Rewrite {
            value: normalized,
            reason: RewriteReason::HeaderCanonicalization,
        }
    } else {
        NormalizationOutcome::Accept(normalized)
    }
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name,
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
