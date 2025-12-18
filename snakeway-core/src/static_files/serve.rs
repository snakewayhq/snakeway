use std::path::PathBuf;
use std::time::SystemTime;

use bytes::Bytes;
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

/// Conditional request headers for cache validation
#[derive(Debug, Default)]
pub struct ConditionalHeaders {
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
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
fn modified_since(
    file_modified: Option<SystemTime>,
    if_modified_since: &str,
) -> bool {
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

    // Build common headers (sent for both 200 and 304)
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    headers.insert(
        http::header::ETAG,
        HeaderValue::from_str(&etag).unwrap(),
    );
    if let Some(ref lm) = last_modified {
        headers.insert(
            http::header::LAST_MODIFIED,
            HeaderValue::from_str(lm).unwrap(),
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

    // Set the Content-Length header based on the file size.
    headers.insert(
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string()).unwrap(),
    );

    // Grab a file handle.
    let mut file = fs::File::open(&path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ServeError::NotFound,
            std::io::ErrorKind::PermissionDenied => ServeError::Forbidden,
            _ => ServeError::Io,
        })?;

    if metadata.len() <= SMALL_FILE_THRESHOLD {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .await
            .map_err(|_| ServeError::Io)?;

        return Ok(StaticResponse {
            status: StatusCode::OK,
            headers,
            body: StaticBody::Bytes(Bytes::from(buf)),
        });
    }

    Ok(StaticResponse {
        status: StatusCode::OK,
        headers,
        body: StaticBody::File(file),
    })
}
