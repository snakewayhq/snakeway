use std::path::PathBuf;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};
use tokio::fs;
use tokio::io::AsyncReadExt;

const MAX_STATIC_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug)]
pub enum ServeError {
    NotFound,
    Forbidden,
    Io,
}

pub struct StaticResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

pub async fn serve_file(path: PathBuf) -> Result<StaticResponse, ServeError> {
    // Check metadata first (size guard)
    let metadata = fs::metadata(&path)
        .await
        .map_err(|_| ServeError::NotFound)?;

    if !metadata.is_file() {
        return Err(ServeError::NotFound);
    }

    if metadata.len() > MAX_STATIC_FILE_SIZE {
        // Prevent memory exhaustion / abuse.
        return Err(ServeError::Forbidden);
    }

    // Open file
    let mut file = fs::File::open(&path)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ServeError::NotFound,
            std::io::ErrorKind::PermissionDenied => ServeError::Forbidden,
            _ => ServeError::Io,
        })?;

    // Read entire file into memory (todo make this more efficient)
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .await
        .map_err(|_| ServeError::Io)?;

    let body = Bytes::from(buf);

    // Guess MIME type
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    headers.insert(
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&body.len().to_string()).unwrap(),
    );

    Ok(StaticResponse {
        status: StatusCode::OK,
        headers,
        body,
    })
}
