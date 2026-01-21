use crate::ctx::request::NormalizedHeaders;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};
use http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashSet;

/// Normalizes HTTP headers according to RFC 9110 and RFC 9112.
///
/// This function performs the following operations:
/// 1. Extracts and processes Connection header tokens (RFC 9110 §7.6.1)
/// 2. Rejects hop-by-hop headers that must not be forwarded
/// 3. Canonicalizes header names to lowercase (RFC 9110 §5.1)
/// 4. Validates header values for proper encoding (RFC 9110 §5.5)
/// 5. Folds duplicate headers with comma-separation (RFC 9110 §5.3)
///
/// # Security Considerations
/// - Rejects headers containing NUL bytes to prevent header injection attacks
/// - Validates all header names and values are properly encoded
/// - Strips hop-by-hop headers to prevent protocol confusion
pub fn normalize_http1_headers(raw: &HeaderMap) -> NormalizationOutcome<NormalizedHeaders> {
    let mut rewritten = false;
    let mut out = HeaderMap::new();

    // RFC 9110 §7.6.1: Extract Connection header tokens to identify additional hop-by-hop headers.
    // The Connection header field allows the sender to list header field names that are only
    // intended for the immediate recipient (hop-by-hop) and should not be forwarded.
    let mut connection_tokens = HashSet::new();
    if let Some(conn) = raw.get("connection") {
        let value = match conn.to_str() {
            Ok(v) => v,
            Err(_) => {
                // RFC 9110 §5.5: Header field values must be valid US-ASCII or encoded properly
                return NormalizationOutcome::reject_for_header_encoding_violation();
            }
        };

        // RFC 9110 §7.6.1: Connection header value is a comma-separated list of tokens
        for token in value.split(',') {
            let token = token.trim().to_ascii_lowercase();
            if !token.is_empty() {
                connection_tokens.insert(token);
            }
        }
    }

    // RFC 9110 §5.1-5.3: Process and normalize each header field
    for (name, value) in raw.iter() {
        let name_str = name.as_str();

        // RFC 9110 §7.6.1: Reject standard hop-by-hop headers and Connection-listed headers.
        // These headers are specific to a single transport-level connection and must not
        // be forwarded by proxies or stored by caches.
        // SECURITY: Lowercased comparison is critical - check against lowercased name_str
        let name_lower = name_str.to_ascii_lowercase();
        if is_standard_hop_by_hop(&name_lower) || connection_tokens.contains(&name_lower) {
            return NormalizationOutcome::Reject {
                reason: RejectReason::HopByHopHeader,
            };
        }

        // RFC 9110 §5.1: Header field names are case-insensitive. Canonicalize to lowercase
        // for consistent processing (following RFC 3986 §6 normalization principles).
        let canonical_name: HeaderName = match name_lower.parse() {
            Ok(h) => h,
            Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
        };

        if name_str != canonical_name.as_str() {
            rewritten = true;
        }

        // RFC 9110 §5.5: Validate header field value encoding
        let value_str = match value.to_str() {
            Ok(v) => v,
            Err(_) => {
                // Non-visible ASCII or invalid UTF-8
                return NormalizationOutcome::reject_for_header_encoding_violation();
            }
        };

        // SECURITY: Reject NUL bytes (0x00) to prevent header injection and smuggling attacks.
        // NUL bytes can cause parsers to terminate strings early, leading to security vulnerabilities.
        if value_str.as_bytes().contains(&0) {
            return NormalizationOutcome::reject_for_header_encoding_violation();
        }

        // RFC 9112 §6.3 and RFC 9110 §5.5: Leading and trailing whitespace (OWS) in field
        // values should be removed. This is part of message parsing normalization.
        let trimmed = value_str.trim();
        if trimmed != value_str {
            rewritten = true;
        }

        let val = match HeaderValue::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
        };

        // RFC 9110 §5.3: Multiple header fields with the same name can be combined into a single
        // field with comma-separated values. This is semantically equivalent for most headers.
        // NOTE: Some headers (e.g., Set-Cookie) have special semantics and should not be folded,
        // but those are response headers. For request headers, comma-folding is generally safe.
        match out.get_mut(&canonical_name) {
            Some(existing) => {
                let merged = match existing.to_str() {
                    Ok(e) => format!("{}, {}", e, trimmed),
                    Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
                };

                let merged_value = match HeaderValue::from_str(&merged) {
                    Ok(v) => v,
                    Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
                };

                *existing = merged_value;
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

/// Checks if a header name is a standard hop-by-hop header per RFC 9110 §7.6.1.
///
/// Hop-by-hop headers are specific to a single transport-level connection and must not
/// be retransmitted by proxies or cached. The standard hop-by-hop headers are defined
/// in RFC 9110 §7.6.1 and include:
/// - Connection: Controls connection-specific options
/// - Keep-Alive: Deprecated, but still recognized for compatibility
/// - Proxy-Authenticate: Proxy authentication challenge
/// - Proxy-Authorization: Proxy authentication credentials
/// - TE: Transfer codings the client is willing to accept (except "trailers")
/// - Trailer: Indicates which headers are present in the trailer
/// - Transfer-Encoding: Encoding transformations applied to the message body
/// - Upgrade: Requests protocol upgrade
///
/// # Arguments
/// * `name` - The header name in lowercase for case-insensitive comparison
///
/// # Security Note
/// This function expects the input to already be lowercased. Callers must ensure
/// case-insensitive comparison by converting header names to lowercase before calling.
fn is_standard_hop_by_hop(name: &str) -> bool {
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
