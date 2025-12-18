use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression;
use http::{HeaderMap, HeaderValue, StatusCode};
use httpdate::{fmt_http_date, parse_http_date};
use tokio::fs;
use tokio::io::AsyncReadExt;

const MAX_STATIC_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB
const SMALL_FILE_THRESHOLD: u64 = 256 * 1024; // 256 KiB

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

/// Minimum size threshold for compression (don't compress tiny files)
const MIN_COMPRESS_SIZE: u64 = 256; // 256 Bytes

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

/// Check if the client accepts gzip encoding
fn accepts_gzip(accept_encoding: &str) -> bool {
    // Parse Accept-Encoding header
    // Format: gzip, deflate, br or gzip;q=1.0, deflate;q=0.5
    for part in accept_encoding.split(',') {
        let encoding = part.split(';').next().unwrap_or("").trim();
        if encoding.eq_ignore_ascii_case("gzip") || encoding == "*" {
            // Check for q=0 which means "not acceptable"
            if let Some(q_part) = part.split(';').nth(1) {
                if let Some(q_value) = q_part.trim().strip_prefix("q=") {
                    if let Ok(q) = q_value.parse::<f32>() {
                        if q == 0.0 {
                            continue;
                        }
                    }
                }
            }
            return true;
        }
    }
    false
}

/// Compress data using gzip
fn gzip_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
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

pub async fn serve_file(
    path: PathBuf,
    conditional: &ConditionalHeaders,
) -> Result<StaticResponse, ServeError> {
    let metadata = fs::metadata(&path)
        .await
        .map_err(|_| ServeError::NotFound)?;

    // Guard against directory traversal attacks.
    if !metadata.is_file() {
        return Err(ServeError::NotFound);
    }

    // Guard against memory exhaustion vulnerability.
    if metadata.len() > MAX_STATIC_FILE_SIZE {
        return Err(ServeError::Forbidden);
    }

    // Get modification time for ETag and Last-Modified
    let modified = metadata.modified().ok();

    // Generate ETag
    let etag = generate_etag(metadata.len(), modified);

    // Format Last-Modified header
    let last_modified = modified.map(fmt_http_date);

    // Check conditional headers for 304 Not Modified response
    let mut not_modified = false;

    // If-None-Match takes precedence over If-Modified-Since (per HTTP spec)
    if let Some(ref if_none_match) = conditional.if_none_match {
        if etag_matches(&etag, if_none_match) {
            not_modified = true;
        }
    } else if let Some(ref if_modified_since) = conditional.if_modified_since {
        if !modified_since(modified, if_modified_since) {
            not_modified = true;
        }
    }

    // Guess MIME type to set the Content-Type header.
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    // Determine if we should compress this response
    let should_compress = metadata.len() >= MIN_COMPRESS_SIZE
        && is_compressible_mime(&mime)
        && conditional
            .accept_encoding
            .as_ref()
            .map(|ae| accepts_gzip(ae))
            .unwrap_or(false);

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
    // This is important for caching proxies
    if is_compressible_mime(&mime) {
        headers.insert(
            http::header::VARY,
            HeaderValue::from_static("Accept-Encoding"),
        );
    }

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
    if metadata.len() <= SMALL_FILE_THRESHOLD {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .await
            .map_err(|_| ServeError::Io)?;

        // Apply gzip compression if appropriate
        if should_compress {
            match gzip_compress(&buf) {
                Ok(compressed) => {
                    // Only use compressed version if it's actually smaller
                    if compressed.len() < buf.len() {
                        headers.insert(
                            http::header::CONTENT_ENCODING,
                            HeaderValue::from_static("gzip"),
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
                Err(_) => {
                    // Compression failed, fall through to uncompressed response
                }
            }
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
