use crate::ctx::request::CanonicalQuery;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};

/// Normalizes HTTP query strings per RFC 3986 and related specifications.
///
/// This function enforces multiple RFC requirements for query string normalization:
///
/// # RFC 3986 (URI Generic Syntax)
/// - **Section 2.1**: Normalizes percent-encoded triplets to uppercase hexadecimal
/// - **Section 2.3**: Decodes percent-encoded unreserved characters (ALPHA/DIGIT/"-"/"."/"_"/"~")
///   to their literal form for canonical comparison
/// - **Section 3.4**: Validates query component structure and encoding
///
/// # RFC 7230 (HTTP/1.1 Message Syntax)
/// - Rejects null bytes in query strings as invalid encoding
/// - Validates percent-encoding syntax (complete triplets with valid hexadecimal digits)
///
/// # Normalization Behavior
/// The function performs the following normalizations:
/// 1. Rejects queries containing null bytes (`\0`)
/// 2. Decodes percent-encoded unreserved characters (e.g., `%41` → `A`)
/// 3. Preserves percent-encoding for reserved and non-ASCII characters
/// 4. Sorts query parameters by key-value pairs for canonical ordering
/// 5. Normalizes remaining percent-encoded sequences to uppercase
///
/// # Returns
/// - `Accept`: Query is already normalized
/// - `Rewrite`: Query was normalized (with reason)
/// - `Reject`: Query contains invalid encoding
pub fn normalize_query(query: &str) -> NormalizationOutcome<CanonicalQuery> {
    if query.is_empty() {
        return NormalizationOutcome::Accept(CanonicalQuery::default());
    }

    if query.as_bytes().contains(&0) {
        return NormalizationOutcome::Reject {
            reason: RejectReason::InvalidQueryEncoding,
        };
    }

    let mut decoded_rewrite = false;
    let mut pairs = Vec::new();

    for part in query.split('&') {
        let (raw_key, raw_val) = match part.split_once('=') {
            Some((k, v)) => (k, v),
            None => (part, ""),
        };

        let (key, key_rewritten) = match percent_decode_unreserved_with_outcome(raw_key) {
            Ok(v) => v,
            Err(r) => return NormalizationOutcome::Reject { reason: r },
        };

        let (val, val_rewritten) = match percent_decode_unreserved_with_outcome(raw_val) {
            Ok(v) => v,
            Err(r) => return NormalizationOutcome::Reject { reason: r },
        };

        decoded_rewrite |= key_rewritten || val_rewritten;
        pairs.push((key, val));
    }

    // Canonical ordering (Phase 3A)
    let mut sorted = pairs.clone();
    sorted.sort();

    let ordering_rewrite = sorted != pairs;
    let rewritten = decoded_rewrite || ordering_rewrite;

    let canonical = CanonicalQuery::new(query, sorted);

    if rewritten {
        NormalizationOutcome::Rewrite {
            value: canonical,
            reason: if decoded_rewrite {
                RewriteReason::PercentDecodeUnreserved
            } else {
                RewriteReason::QueryCanonicalization
            },
        }
    } else {
        NormalizationOutcome::Accept(canonical)
    }
}

fn percent_decode_unreserved_with_outcome(input: &str) -> Result<(String, bool), RejectReason> {
    let decoded =
        percent_decode_unreserved(input).map_err(|_| RejectReason::InvalidPercentEncoding)?;

    Ok((decoded.clone(), decoded != input))
}

/// Decodes percent-encoded sequences that represent unreserved characters per RFC 3986 Section 2.3.
///
/// RFC 3986 defines unreserved characters as: ALPHA / DIGIT / "-" / "." / "_" / "~".
/// This function enforces the normalization requirement that percent-encoded triplets for these
/// characters SHOULD be decoded to their literal form for URI comparison purposes.
///
/// Percent-encoded sequences representing reserved or other characters are preserved as-is,
/// ensuring that the semantic meaning of the URI is not altered during normalization.
///
/// # Security
/// - Rejects malformed percent-encoding sequences (e.g. incomplete or non-hex triplets)
/// - Restricts decoding to ASCII-range percent-encoded bytes (0–127)
/// - Normalizes preserved percent-encoded sequences to uppercase per RFC 3986 Section 2.1
fn percent_decode_unreserved(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(());
                }

                let hex = &input[i + 1..i + 3];
                let val = u8::from_str_radix(hex, 16).map_err(|_| ())?;

                // Security: Only process valid ASCII bytes (0-127).
                // Casting non-ASCII bytes (128-255) to char is unsafe and can create invalid Unicode.
                // Non-ASCII bytes must remain percent-encoded per RFC 3986.
                if val > 127 {
                    // Preserve as percent-encoded, normalized to uppercase per RFC 3986 Section 2.1
                    out.push('%');
                    out.push_str(&format!("{:02X}", val));
                    i += 3;
                    continue;
                }

                let ch = val as char;

                // Decode unreserved characters only (RFC 3986 Section 2.3)
                // Unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '_' | '~') {
                    out.push(ch);
                } else {
                    // Preserve reserved and other characters as percent-encoded,
                    // normalized to uppercase per RFC 3986 Section 2.1
                    out.push('%');
                    out.push_str(&format!("{:02X}", val));
                }

                i += 3;
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }

    Ok(out)
}
