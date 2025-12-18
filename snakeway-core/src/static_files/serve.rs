use std::path::PathBuf;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};
use tokio::fs;

const MAX_STATIC_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB

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

pub async fn serve_file(path: PathBuf) -> Result<StaticResponse, ServeError> {
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

    // Grab a file handle.
    let file = fs::File::open(&path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ServeError::NotFound,
            std::io::ErrorKind::PermissionDenied => ServeError::Forbidden,
            _ => ServeError::Io,
        })?;

    // Guess MIME type to set the Content-Type header.
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );

    // Set the Content-Length header based on the file size.
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

// pub async fn serve_file_deprecated(path: PathBuf) -> Result<StaticResponse, ServeError> {
//     // Check metadata first (size guard)
//     let metadata = fs::metadata(&path)
//         .await
//         .map_err(|_| ServeError::NotFound)?;
//
//     if !metadata.is_file() {
//         return Err(ServeError::NotFound);
//     }
//
//     if metadata.len() > MAX_STATIC_FILE_SIZE {
//         // Prevent memory exhaustion / abuse.
//         return Err(ServeError::Forbidden);
//     }
//
//     // Open file
//     let mut file = fs::File::open(&path)
//         .await
//         .map_err(|err| match err.kind() {
//             std::io::ErrorKind::NotFound => ServeError::NotFound,
//             std::io::ErrorKind::PermissionDenied => ServeError::Forbidden,
//             _ => ServeError::Io,
//         })?;
//
//     // Read entire file into memory (todo make this more efficient)
//     let mut buf = Vec::new();
//     file.read_to_end(&mut buf)
//         .await
//         .map_err(|_| ServeError::Io)?;
//
//     let body = Bytes::from(buf);
//
//     // Guess MIME type
//     let mime = mime_guess::from_path(&path).first_or_octet_stream();
//
//     let mut headers = HeaderMap::new();
//     headers.insert(
//         http::header::CONTENT_TYPE,
//         HeaderValue::from_str(mime.as_ref()).unwrap(),
//     );
//     headers.insert(
//         http::header::CONTENT_LENGTH,
//         HeaderValue::from_str(&body.len().to_string()).unwrap(),
//     );
//
//     Ok(StaticResponse {
//         status: StatusCode::OK,
//         headers,
//         body,
//     })
// }
