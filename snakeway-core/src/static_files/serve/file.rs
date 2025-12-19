use std::path::PathBuf;

use crate::config::{StaticCachePolicy, StaticFileConfig};
use crate::static_files::serve::compression::{
    brotli_compress, gzip_compress, is_compressible_mime, preferred_encoding,
    response_varies_by_encoding,
};
use crate::static_files::serve::etag::{etag_matches, generate_etag, modified_since};

use crate::static_files::serve::cache::apply_cache_headers;

use crate::static_files::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};
use httpdate::fmt_http_date;
use tokio::fs;
use tokio::io::AsyncReadExt;

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
        headers.insert(http::header::CONTENT_LENGTH, HeaderValue::from_static("0"));
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
