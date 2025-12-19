use std::io::Write;

use crate::config::StaticFileConfig;
use flate2::Compression;
use flate2::write::GzEncoder;

/// Check if a MIME type is compressible (text-based or common web formats)
pub(crate) fn is_compressible_mime(mime: &mime_guess::Mime) -> bool {
    let type_ = mime.type_();
    let subtype = mime.subtype();

    // Text types are always compressible
    if type_ == "text" {
        return true;
    }

    // Application types that are text-based
    if type_ == "application" {
        let subtype_str = subtype.as_str();
        return matches!(
            subtype_str,
            "json"
                | "javascript"
                | "x-javascript"
                | "xml"
                | "xhtml+xml"
                | "rss+xml"
                | "atom+xml"
                | "svg+xml"
                | "x-www-form-urlencoded"
                | "wasm"
        );
    }

    // SVG images
    if type_ == "image" && subtype == "svg+xml" {
        return true;
    }

    false
}

/// Parse quality value from Accept-Encoding part (e.g., "gzip;q=0.5" -> 0.5)
fn parse_quality(part: &str) -> f32 {
    part.split(';')
        .nth(1)
        .and_then(|s| s.trim().strip_prefix("q="))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0)
}

/// Check if the client accepts a specific encoding and return its quality value
pub(crate) fn accepts_encoding(accept_encoding: &str, encoding_name: &str) -> Option<f32> {
    for part in accept_encoding.split(',') {
        let encoding = part.split(';').next().unwrap_or("").trim();
        if encoding.eq_ignore_ascii_case(encoding_name) || encoding == "*" {
            let q = parse_quality(part);
            if q == 0.0 {
                return None; // q=0 means "not acceptable"
            }
            return Some(q);
        }
    }
    None
}

/// Determine the preferred compression encoding based on Accept-Encoding header
/// Returns "br" for brotli, "gzip" for gzip, or None for no compression
pub(crate) fn preferred_encoding(accept_encoding: &str) -> Option<&'static str> {
    let br_quality = accepts_encoding(accept_encoding, "br");
    let gzip_quality = accepts_encoding(accept_encoding, "gzip");

    match (br_quality, gzip_quality) {
        (Some(br_q), Some(gzip_q)) => {
            // Prefer brotli if quality is equal or higher
            if br_q >= gzip_q {
                Some("br")
            } else {
                Some("gzip")
            }
        }
        (Some(_), None) => Some("br"),
        (None, Some(_)) => Some("gzip"),
        (None, None) => None,
    }
}

/// Compress data using gzip
pub(crate) fn gzip_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
}

/// Compress data using brotli
pub(crate) fn brotli_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    // Parameters: quality (0-11), lg_window_size (10-24)
    // Using quality 4 for a good balance between speed and compression
    let params = brotli::enc::BrotliEncoderParams {
        quality: 4,
        lgwin: 22,
        ..Default::default()
    };
    brotli::enc::BrotliCompress(&mut std::io::Cursor::new(data), &mut output, &params)?;
    Ok(output)
}

/// Check if the response should vary based on Accept-Encoding header.
/// This is an important header for caching proxies.
pub(crate) fn response_varies_by_encoding(
    mime: &mime_guess::Mime,
    size: u64,
    cfg: &StaticFileConfig,
) -> bool {
    if !is_compressible_mime(mime) {
        return false;
    }

    if cfg.enable_brotli && size >= cfg.min_brotli_size {
        return true;
    }

    if cfg.enable_gzip && size >= cfg.min_gzip_size {
        return true;
    }

    false
}
