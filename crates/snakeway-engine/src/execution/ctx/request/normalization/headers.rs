use crate::execution::ctx::request::NormalizedHeaders;
use crate::execution::ctx::request::normalization::http1_headers::normalize_http1_headers;
use crate::execution::ctx::request::normalization::http2_headers::normalize_http2_headers;
use crate::execution::ctx::request::normalization::{
    NormalizationOutcome, ProtocolNormalizationMode,
};
use http::HeaderMap;

/// Normalizes HTTP headers according to the appropriate protocol specification.
///
/// This function dispatches to protocol-specific normalization based on the HTTP version:
/// - HTTP/1.x: Enforces RFC 9110 (HTTP Semantics) and RFC 9112 (HTTP/1.1)
/// - HTTP/2: Enforces RFC 9110 (HTTP Semantics) and RFC 9113 (HTTP/2)
///
/// Both protocol modes validate header encoding, reject hop-by-hop headers, canonicalize
/// header names and values, and fold duplicate headers according to their respective RFCs.
pub fn normalize_headers(
    raw: &HeaderMap,
    protocol_mode: &ProtocolNormalizationMode,
) -> NormalizationOutcome<NormalizedHeaders> {
    match protocol_mode {
        ProtocolNormalizationMode::Http1 => normalize_http1_headers(raw),
        ProtocolNormalizationMode::Http2 => normalize_http2_headers(raw),
    }
}

#[cfg(test)]
mod tests {
    use http::{HeaderName, HeaderValue};

    // Parse and universal tests (Should be very few here).
    #[test]
    fn reject_nul_in_header_value_at_parse_time() {
        assert!(HeaderValue::from_bytes(b"abc\0def").is_err());
    }

    #[test]
    fn header_name_auto_lowercased_at_parse_time() {
        let expected = "host";

        let result = HeaderName::from_bytes(b"Host").unwrap();

        assert_eq!(result.as_str(), expected);
    }
}
