use std::time::SystemTime;

use httpdate::parse_http_date;

/// Generate an ETag from file size and modification time.
/// Format: "size-mtime_secs" (weak ETag using W/ prefix)
pub(crate) fn generate_etag(size: u64, modified: Option<SystemTime>) -> String {
    let mtime_secs = modified
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("W/\"{:x}-{:x}\"", size, mtime_secs)
}

/// Check if the ETag matches the If-None-Match header value.
/// Handles both strong and weak comparison (weak by default for our ETags).
pub(crate) fn etag_matches(etag: &str, if_none_match: &str) -> bool {
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
pub(crate) fn modified_since(file_modified: Option<SystemTime>, if_modified_since: &str) -> bool {
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
