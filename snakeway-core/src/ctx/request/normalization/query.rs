use crate::ctx::request::CanonicalQuery;
use crate::ctx::request::normalization::{NormalizationOutcome, RejectReason, RewriteReason};

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
                let ch = val as char;

                // Decode unreserved only (RFC 3986)
                if ch.is_ascii_alphanumeric() || "-._~".contains(ch) {
                    out.push(ch);
                } else {
                    out.push('%');
                    out.push_str(hex);
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
