use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config::{StaticCachePolicy, StaticFileConfig};
use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression;
use http::{HeaderMap, HeaderValue, StatusCode};
use httpdate::{fmt_http_date, parse_http_date};
use tokio::fs;
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub enum ServeError {
    NotFound,
    Forbidden,
    Io,
}

pub enum StaticBody {
    Empty,
    /// Useful for tiny files/errors.
    Bytes(Bytes),
    /// Useful for large files that require streaming from disk.
    File(fs::File),
}

pub struct StaticResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: StaticBody,
}

/// Conditional request headers for cache validation and content negotiation
#[derive(Debug, Default)]
pub struct ConditionalHeaders {
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
    pub accept_encoding: Option<String>,
}

/// Check if a MIME type is compressible (text-based or common web formats)
fn is_compressible_mime(mime: &mime_guess::Mime) -> bool {
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
fn accepts_encoding(accept_encoding: &str, encoding_name: &str) -> Option<f32> {
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
fn preferred_encoding(accept_encoding: &str) -> Option<&'static str> {
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
fn gzip_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
}

/// Compress data using brotli
fn brotli_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
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

/// Generate an ETag from file size and modification time.
/// Format: "size-mtime_secs" (weak ETag using W/ prefix)
fn generate_etag(size: u64, modified: Option<SystemTime>) -> String {
    let mtime_secs = modified
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("W/\"{:x}-{:x}\"", size, mtime_secs)
}

/// Check if the ETag matches the If-None-Match header value.
/// Handles both strong and weak comparison (weak by default for our ETags).
fn etag_matches(etag: &str, if_none_match: &str) -> bool {
    // Handle "*" which matches any ETag
    if if_none_match.trim() == "*" {
        return true;
    }

    // Parse comma-separated list of ETags
    for candidate in if_none_match.split(',') {
        let candidate = candidate.trim();
        // Strip W/ prefix for weak comparison
        let candidate_value = candidate.strip_prefix("W/").unwrap_or(candidate);
        let etag_value = etag.strip_prefix("W/").unwrap_or(etag);
        if candidate_value == etag_value {
            return true;
        }
    }
    false
}

/// Check if the file has been modified since the given date.
fn modified_since(file_modified: Option<SystemTime>, if_modified_since: &str) -> bool {
    let file_time = match file_modified {
        Some(t) => t,
        None => return true, // Unknown mtime, assume modified
    };

    let since_time = match parse_http_date(if_modified_since) {
        Ok(t) => t,
        Err(_) => return true, // Invalid header, assume modified
    };

    // A simple comparison like file_time > since_time does not work,
    // because HTTP dates have 1-second resolution.
    // Treat sub-second differences as NOT modified.
    match file_time.duration_since(since_time) {
        Ok(delta) => delta.as_secs() >= 1,
        Err(_) => false, // file_time <= since_time, not modified
    }
}

fn apply_cache_headers(headers: &mut HeaderMap, policy: &StaticCachePolicy) {
    let mut value = String::new();

    if policy.public {
        value.push_str("public");
    } else {
        value.push_str("private");
    }

    value.push_str(&format!(", max-age={}", policy.max_age));

    if policy.immutable {
        value.push_str(", immutable");
    }

    headers.insert(
        http::header::CACHE_CONTROL,
        HeaderValue::from_str(&value).unwrap(),
    );
}

/// Check if the response should vary based on Accept-Encoding header.
/// This is an important header for caching proxies.
fn response_varies_by_encoding(mime: &mime_guess::Mime, size: u64, cfg: &StaticFileConfig) -> bool {
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

pub async fn serve_file(
    path: PathBuf,
    conditional: &ConditionalHeaders,
    static_config: &StaticFileConfig,
    cache_policy: &StaticCachePolicy,
) -> Result<StaticResponse, ServeError> {
    let metadata = fs::metadata(&path)
        .await
        .map_err(|_| ServeError::NotFound)?;

    // Guard against directory traversal attacks.
    if !metadata.is_file() {
        return Err(ServeError::NotFound);
    }

    // Guard against memory exhaustion vulnerability.
    if metadata.len() > static_config.max_file_size {
        return Err(ServeError::Forbidden);
    }

    // Get modification time for ETag and Last-Modified
    let modified = metadata.modified().ok();

    // Generate ETag
    let etag = generate_etag(metadata.len(), modified);

    // Format Last-Modified header
    let last_modified = modified.map(fmt_http_date);

    // Check conditional headers for 304 Not Modified response
    let not_modified = match (
        conditional.if_none_match.as_deref(),
        conditional.if_modified_since.as_deref(),
    ) {
        (Some(inm), _) => etag_matches(&etag, inm),
        (None, Some(ims)) => !modified_since(modified, ims),
        _ => false,
    };

    // Guess MIME type to set the Content-Type header.
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    // Determine the preferred compression encoding (brotli > gzip)
    let preferred_enc = if is_compressible_mime(&mime) {
        conditional.accept_encoding.as_ref().and_then(|ae| {
            let size = metadata.len();

            // Determine what encodings are allowed based on size
            let br_allowed = static_config.enable_brotli && size >= static_config.min_brotli_size;
            let gzip_allowed = static_config.enable_gzip && size >= static_config.min_gzip_size;

            match preferred_encoding(ae) {
                Some("br") if br_allowed => Some("br"),
                Some("br") if !br_allowed && gzip_allowed => Some("gzip"),
                Some("gzip") if gzip_allowed => Some("gzip"),
                _ => None,
            }
        })
    } else {
        None
    };

    // Build common headers (sent for both 200 and 304)
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    headers.insert(http::header::ETAG, HeaderValue::from_str(&etag).unwrap());
    if let Some(ref lm) = last_modified {
        headers.insert(
            http::header::LAST_MODIFIED,
            HeaderValue::from_str(lm).unwrap(),
        );
    }

    // Add Vary header to indicate response varies based on Accept-Encoding
    if response_varies_by_encoding(&mime, metadata.len(), static_config) {
        headers.insert(
            http::header::VARY,
            HeaderValue::from_static("Accept-Encoding"),
        );
    }

    // Apply cache policy headers
    apply_cache_headers(&mut headers, cache_policy);

    // Return 304 Not Modified if conditions are met
    if not_modified {
        return Ok(StaticResponse {
            status: StatusCode::NOT_MODIFIED,
            headers,
            body: StaticBody::Empty,
        });
    }

    // Grab a file handle.
    let mut file = fs::File::open(&path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ServeError::NotFound,
            std::io::ErrorKind::PermissionDenied => ServeError::Forbidden,
            _ => ServeError::Io,
        })?;

    // For small files, read into memory (and optionally compress)
    if metadata.len() <= static_config.small_file_threshold {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .await
            .map_err(|_| ServeError::Io)?;

        // Apply compression if appropriate (prefer brotli, fallback to gzip).
        if let Some(encoding) = preferred_enc {
            let compress_result = match encoding {
                "br" => brotli_compress(&buf),
                "gzip" => gzip_compress(&buf),
                _ => Err(std::io::Error::other("unknown encoding")),
            };

            if let Ok(compressed) = compress_result {
                // Only use compressed version if it's actually smaller.
                if compressed.len() < buf.len() {
                    headers.insert(
                        http::header::CONTENT_ENCODING,
                        HeaderValue::from_static(encoding),
                    );
                    headers.insert(
                        http::header::CONTENT_LENGTH,
                        HeaderValue::from_str(&compressed.len().to_string()).unwrap(),
                    );
                    return Ok(StaticResponse {
                        status: StatusCode::OK,
                        headers,
                        body: StaticBody::Bytes(Bytes::from(compressed)),
                    });
                }
            }
            // Compression failed or didn't reduce size, fall through to uncompressed response...
        }

        // Uncompressed response
        headers.insert(
            http::header::CONTENT_LENGTH,
            HeaderValue::from_str(&buf.len().to_string()).unwrap(),
        );
        return Ok(StaticResponse {
            status: StatusCode::OK,
            headers,
            body: StaticBody::Bytes(Bytes::from(buf)),
        });
    }

    // For large files, stream without compression
    // (streaming compression would require async-compression crate)
    headers.insert(
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string()).unwrap(),
    );

    Ok(StaticResponse {
        status: StatusCode::OK,
        headers,
        body: StaticBody::File(file),
    })
}
