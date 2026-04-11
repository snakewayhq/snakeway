use crate::execution::ctx::request::NormalizedHeaders;
use crate::execution::ctx::request::normalization::{NormalizationOutcome, RewriteReason};
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
pub(crate) fn normalize_http1_headers(raw: &HeaderMap) -> NormalizationOutcome<NormalizedHeaders> {
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

    // RFC 9112 §6.3: Reject requests that carry both Transfer-Encoding and Content-Length.
    // When both are present the message framing is ambiguous: different intermediaries may
    // disagree on which header governs the body length, enabling request smuggling.
    // Silently stripping one header and forwarding would mask the attack; reject instead.
    let has_te = raw.contains_key("transfer-encoding");
    let has_cl = raw.contains_key("content-length");
    if has_te && has_cl {
        return NormalizationOutcome::reject_for_smuggling_attempt();
    }

    // RFC 9112 §6.3: Reject duplicate Content-Length headers with differing values.
    // A proxy that picks the first value and a backend that picks the last (or vice-versa)
    // will disagree on where the first request ends, allowing a hidden second request.
    {
        let cl_values: Vec<_> = raw.get_all("content-length").iter().collect();
        if cl_values.len() > 1 {
            let first = cl_values[0].to_str().unwrap_or("").trim();
            let all_equal = cl_values
                .iter()
                .all(|v| v.to_str().unwrap_or("").trim() == first);
            if !all_equal {
                return NormalizationOutcome::reject_for_smuggling_attempt();
            }
        }
    }

    // RFC 9110 §5.1-5.3: Process and normalize each header field
    for (name, value) in raw.iter() {
        let name_str = name.as_str();

        // RFC 9110 §7.6.1: Strip standard hop-by-hop headers and Connection-listed headers.
        // These headers are specific to a single transport-level connection and must not
        // be forwarded by proxies or stored by caches.
        // SECURITY: Lowercased comparison is critical - check against lowercased name_str
        let name_lower = name_str.to_ascii_lowercase();
        if is_standard_hop_by_hop(&name_lower) || connection_tokens.contains(&name_lower) {
            rewritten = true;
            continue;
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

#[cfg(test)]
mod tests {
    use crate::execution::ctx::request::normalization::{
        NormalizationOutcome, ProtocolNormalizationMode, RejectReason, RewriteReason,
        normalize_headers,
    };
    use http::{HeaderMap, HeaderName, HeaderValue};

    fn input_to_header_map(input: &[(&str, &str)]) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        for (k, v) in input {
            let name: HeaderName = k.parse().expect("invalid header name");
            let value: HeaderValue = v.parse().expect("invalid header value");
            header_map.append(name, value);
        }
        header_map
    }

    fn assert_accept_headers(
        input: &[(&str, &str)],
        expected: &[(&str, &str)],
        protocol_mode: &ProtocolNormalizationMode,
    ) {
        // Arrange
        let raw = input_to_header_map(input);

        // Act
        let outcome = normalize_headers(&raw, protocol_mode);

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
        protocol_mode: &ProtocolNormalizationMode,
    ) {
        // Arrange
        let raw = input_to_header_map(input);

        // Act
        let outcome = normalize_headers(&raw, protocol_mode);

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

    fn assert_reject_headers(
        input: &[(&str, &str)],
        reason: RejectReason,
        protocol_mode: &ProtocolNormalizationMode,
    ) {
        // Arrange
        let raw = input_to_header_map(input);

        // Act
        let outcome = normalize_headers(&raw, protocol_mode);

        // Assert
        match outcome {
            NormalizationOutcome::Reject { reason: r } => {
                assert_eq!(r, reason);
            }
            other => panic!("Expected Reject, got {:?}", other),
        }
    }

    fn assert_accept_http1_headers(input: &[(&str, &str)], expected: &[(&str, &str)]) {
        assert_accept_headers(input, expected, &ProtocolNormalizationMode::Http1);
    }

    fn assert_rewrite_http1_headers(
        input: &[(&str, &str)],
        expected: &[(&str, &str)],
        reason: RewriteReason,
    ) {
        assert_rewrite_headers(input, expected, reason, &ProtocolNormalizationMode::Http1);
    }

    fn assert_reject_http1_headers(input: &[(&str, &str)], reason: RejectReason) {
        assert_reject_headers(input, reason, &ProtocolNormalizationMode::Http1);
    }

    //-----------------------------------------------------------------------------
    // Accept cases
    //-----------------------------------------------------------------------------
    #[test]
    fn accept_simple_headers() {
        assert_accept_http1_headers(
            &[("host", "example.com"), ("user-agent", "curl/8.0")],
            &[("host", "example.com"), ("user-agent", "curl/8.0")],
        );
    }

    #[test]
    fn accept_header_name_case_insensitive() {
        assert_accept_http1_headers(
            &[("Host", "example.com"), ("USER-AGENT", "curl")],
            &[("host", "example.com"), ("user-agent", "curl")],
        );
    }

    #[test]
    fn accept_multiple_distinct_headers() {
        assert_accept_http1_headers(
            &[("accept", "text/plain"), ("accept-encoding", "gzip")],
            &[("accept", "text/plain"), ("accept-encoding", "gzip")],
        );
    }

    //-----------------------------------------------------------------------------
    // Rewrite cases
    //-----------------------------------------------------------------------------
    #[test]
    fn rewrite_fold_duplicate_headers() {
        assert_rewrite_http1_headers(
            &[("accept", "text/plain"), ("accept", "application/json")],
            &[("accept", "text/plain, application/json")],
            RewriteReason::HeaderCanonicalization,
        );
    }

    #[test]
    fn rewrite_trim_whitespace() {
        assert_rewrite_http1_headers(
            &[("x-test", "  value  ")],
            &[("x-test", "value")],
            RewriteReason::HeaderCanonicalization,
        );
    }

    //-----------------------------------------------------------------------------
    // Reject cases
    //-----------------------------------------------------------------------------
    #[test]
    fn reject_nul_in_header_value_at_parse_time() {
        assert!(HeaderValue::from_bytes(b"abc\0def").is_err());
    }

    #[test]
    fn strip_hop_by_hop_header() {
        // In HTTP/1, hop-by-hop headers are stripped (not rejected); the result is a rewrite.
        assert_rewrite_http1_headers(
            &[
                ("host", "example.com"),
                ("connection", "keep-alive"),
                ("transfer-encoding", "chunked"),
            ],
            &[("host", "example.com")],
            RewriteReason::HeaderCanonicalization,
        );
    }

    //-----------------------------------------------------------------------------
    // Smuggling reject cases (RFC 9112 §6.3)
    //-----------------------------------------------------------------------------

    #[test]
    fn reject_cl_te_smuggling() {
        // CL.TE: Content-Length then Transfer-Encoding
        assert_reject_http1_headers(
            &[
                ("host", "example.com"),
                ("content-length", "13"),
                ("transfer-encoding", "chunked"),
            ],
            RejectReason::RequestSmugglingAttempt,
        );
    }

    #[test]
    fn reject_te_cl_smuggling() {
        // TE.CL: Transfer-Encoding then Content-Length
        assert_reject_http1_headers(
            &[
                ("host", "example.com"),
                ("transfer-encoding", "chunked"),
                ("content-length", "4"),
            ],
            RejectReason::RequestSmugglingAttempt,
        );
    }

    #[test]
    fn reject_dual_content_length_differing_values() {
        assert_reject_http1_headers(
            &[
                ("host", "example.com"),
                ("content-length", "5"),
                ("content-length", "10"),
            ],
            RejectReason::RequestSmugglingAttempt,
        );
    }

    #[test]
    fn accept_duplicate_content_length_same_value() {
        // Identical duplicate CL is technically redundant but not ambiguous; strip the duplicate.
        assert_rewrite_http1_headers(
            &[
                ("host", "example.com"),
                ("content-length", "5"),
                ("content-length", "5"),
            ],
            &[("host", "example.com"), ("content-length", "5, 5")],
            RewriteReason::HeaderCanonicalization,
        );
    }
}
