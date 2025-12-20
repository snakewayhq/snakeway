use std::path::PathBuf;

use crate::config::{StaticCachePolicy, StaticFileConfig};
use crate::static_files::render::compression::{
    CompressionEncoding, brotli_compress, gzip_compress, is_compressible_mime, preferred_encoding,
    response_varies_by_encoding,
};
use crate::static_files::render::etag::{etag_matches, generate_etag, modified_since};

use crate::static_files::render::cache::apply_cache_headers;

use crate::static_files::render::range::parse_range_header;
use crate::static_files::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};
use httpdate::fmt_http_date;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub async fn render_file(
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
                Some(CompressionEncoding::Brotli) if br_allowed => {
                    Some(CompressionEncoding::Brotli)
                }
                Some(CompressionEncoding::Brotli) if !br_allowed && gzip_allowed => {
                    Some(CompressionEncoding::Gzip)
                }
                Some(CompressionEncoding::Gzip) if gzip_allowed => Some(CompressionEncoding::Gzip),
                _ => None,
            }
        })
    } else {
        None
    };

    // Build common headers (sent for both 200 and 304)
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::ACCEPT_RANGES,
        HeaderValue::from_static("bytes"),
    );
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

    // compute the range header
    let mut range = conditional
        .range
        .as_deref()
        .and_then(|r| parse_range_header(r, metadata.len()));

    if preferred_enc.is_some() {
        range = None;
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
        // Use a pre-allocated vec for better performance.
        // This is NOT a micro optimization - it yields a 30% rps increase.
        let mut buf = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buf)
            .await
            .map_err(|_| ServeError::Io)?;

        // Apply compression if appropriate (prefer brotli, fallback to gzip).
        if let Some(encoding) = preferred_enc {
            let compress_result = match encoding {
                CompressionEncoding::Brotli => brotli_compress(&buf),
                CompressionEncoding::Gzip => gzip_compress(&buf),
                _ => Err(std::io::Error::other(CompressionEncoding::Unknown.as_str())),
            };

            if let Ok(compressed) = compress_result {
                // Only use compressed version if it's actually smaller.
                if compressed.len() < buf.len() {
                    headers.insert(
                        http::header::CONTENT_ENCODING,
                        HeaderValue::from_static(encoding.as_str()),
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

        // Apply range header, if the content is not compressed and the header exists.
        if let Some(range) = range {
            let slice = &buf[range.start as usize..=range.end as usize];

            headers.insert(
                http::header::CONTENT_RANGE,
                HeaderValue::from_str(&format!(
                    "bytes {}-{}/{}",
                    range.start,
                    range.end,
                    metadata.len()
                ))
                .unwrap(),
            );

            headers.insert(
                http::header::CONTENT_LENGTH,
                HeaderValue::from_str(&slice.len().to_string()).unwrap(),
            );

            return Ok(StaticResponse {
                status: StatusCode::PARTIAL_CONTENT,
                headers,
                body: StaticBody::Bytes(Bytes::copy_from_slice(slice)),
            });
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
    // todo: streaming compression would require async-compression crate
    if let Some(range) = range {
        file.seek(std::io::SeekFrom::Start(range.start))
            .await
            .map_err(|_| ServeError::Io)?;

        let remaining = range.end - range.start + 1;

        headers.insert(
            http::header::CONTENT_RANGE,
            HeaderValue::from_str(&format!(
                "bytes {}-{}/{}",
                range.start,
                range.end,
                metadata.len()
            ))
            .unwrap(),
        );

        headers.insert(
            http::header::CONTENT_LENGTH,
            HeaderValue::from_str(&remaining.to_string()).unwrap(),
        );

        return Ok(StaticResponse {
            status: StatusCode::PARTIAL_CONTENT,
            headers,
            body: StaticBody::RangedFile { file, remaining },
        });
    }

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
