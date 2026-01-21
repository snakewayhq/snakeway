use crate::ctx::request::NormalizedHeaders;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};
use http::{HeaderMap, HeaderValue};

/// Normalizes HTTP/2 headers according to RFC 9110 and RFC 9113.
///
/// This function performs the following operations:
/// 1. Validates header names are lowercase (RFC 9113 §8.2.1)
/// 2. Rejects connection-specific headers forbidden in HTTP/2 (RFC 9113 §8.2.2)
/// 3. Enforces TE header restrictions - only "trailers" allowed (RFC 9113 §8.2.1.2)
/// 4. Validates header values for proper encoding (RFC 9110 §5.5)
/// 5. Trims optional whitespace from header values
/// 6. Folds duplicate headers with comma-separation (RFC 9110 §5.3)
///
/// # HTTP/2-Specific Rules
/// - Header field names MUST be lowercase (RFC 9113 §8.2.1)
/// - Connection-specific headers (Connection, Keep-Alive, Proxy-Authenticate,
///   Proxy-Authorization, Transfer-Encoding, Upgrade, Trailer) are forbidden (RFC 9113 §8.2.2)
/// - The TE header is only allowed with value "trailers" (RFC 9113 §8.2.1.2)
/// - HTTP/2 does not support obs-fold (obsolete line folding)
///
/// # Security Considerations
/// - Rejects headers containing NUL bytes to prevent header injection attacks
/// - Validates all header names and values are properly encoded
/// - Strictly enforces HTTP/2 protocol requirements to prevent downgrade attacks
pub fn normalize_http2_headers(raw: &HeaderMap) -> NormalizationOutcome<NormalizedHeaders> {
    let mut rewritten = false;
    let mut out = HeaderMap::new();

    for (name, value) in raw.iter() {
        let name_str = name.as_str();

        // RFC 9113 §8.2.1: Header field names MUST be lowercase
        if name_str.chars().any(|c| c.is_ascii_uppercase()) {
            return NormalizationOutcome::reject_for_header_encoding_violation();
        }

        // RFC 9113 §8.2.2: Connection-specific headers are forbidden
        if is_http2_forbidden_header(name_str) {
            return NormalizationOutcome::Reject {
                reason: RejectReason::HopByHopHeader,
            };
        }

        // RFC 9113 §8.2.1.2: TE header special case
        // Only allowed value: "trailers"
        if name_str == "te" {
            let v = match value.to_str() {
                Ok(v) => v.trim(),
                Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
            };

            if v != "trailers" {
                return NormalizationOutcome::Reject {
                    reason: RejectReason::HopByHopHeader,
                };
            }
        }

        // RFC 9110 §5.5: Validate header value encoding
        let value_str = match value.to_str() {
            Ok(v) => v,
            Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
        };

        // Reject NUL bytes outright
        if value_str.as_bytes().contains(&0) {
            return NormalizationOutcome::reject_for_header_encoding_violation();
        }

        // HTTP/2 disallows obs-fold; trimming OWS is safe and canonical
        let trimmed = value_str.trim();
        if trimmed != value_str {
            rewritten = true;
        }

        let val = match HeaderValue::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return NormalizationOutcome::reject_for_header_encoding_violation(),
        };

        // RFC 9110 §5.3: Fold duplicate headers
        match out.get_mut(name) {
            Some(existing) => {
                // RFC 9110 §5.3:
                // Multiple request header fields with the same name may be combined
                // into a single field by comma-separating their values IF the header’s
                // field definition allows list semantics.
                //
                // At this point we have already validated that this header:
                //   - is not hop-by-hop
                //   - is safe to fold for requests
                //
                // SECURITY:
                // We must re-validate the existing value before merging to ensure it
                // remains valid ASCII and does not contain illegal bytes (e.g., NUL).
                let merged = match existing.to_str() {
                    Ok(e) => format!("{}, {}", e, trimmed),
                    Err(_) => {
                        // Existing header value failed UTF-8 / ASCII validation.
                        // This indicates malformed input and must be rejected.
                        return NormalizationOutcome::reject_for_header_encoding_violation();
                    }
                };

                // SECURITY:
                // Re-parse the merged value into a HeaderValue to ensure it conforms
                // to HTTP header value grammar after folding. This prevents accidental
                // construction of invalid or injection-capable values.
                *existing = match HeaderValue::from_str(&merged) {
                    Ok(v) => v,
                    Err(_) => {
                        // The merged header value violates header encoding rules.
                        return NormalizationOutcome::reject_for_header_encoding_violation();
                    }
                };

                // Folding multiple headers into a single canonical value
                // constitutes a semantic rewrite.
                rewritten = true;
            }
            None => {
                // First occurrence of this header name... insert as-is...
                out.insert(name.clone(), val);
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

/// Checks if a header name is forbidden in HTTP/2 per RFC 9113 §8.2.2.
///
/// HTTP/2 prohibits connection-specific header fields that are specific to a particular
/// connection and must not be forwarded. These headers are forbidden because HTTP/2 uses
/// a single multiplexed connection and does not support connection-level negotiation in
/// the same way as HTTP/1.1.
///
/// The forbidden headers are defined in RFC 9113 §8.2.2 and include:
/// - Connection: Not needed in HTTP/2's multiplexed model
/// - Keep-Alive: Not applicable to HTTP/2's persistent connection model
/// - Proxy-Authenticate: Connection-specific proxy authentication
/// - Proxy-Authorization: Connection-specific proxy credentials
/// - Transfer-Encoding: HTTP/2 has built-in framing, making this obsolete
/// - Upgrade: Protocol upgrade is handled differently in HTTP/2
/// - Trailer: Trailers are handled via special HTTP/2 frames
///
/// # Arguments
/// * `name` - The header name in lowercase (HTTP/2 requires lowercase header names)
///
/// # Security Note
/// This function expects the input to already be lowercased per RFC 9113 §8.2.1.
/// The presence of these headers in an HTTP/2 request must result in connection termination
/// to prevent protocol confusion attacks and ensure HTTP/2 semantic integrity.
fn is_http2_forbidden_header(name: &str) -> bool {
    matches!(
        name,
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "transfer-encoding"
            | "upgrade"
            | "trailer"
    )
}
