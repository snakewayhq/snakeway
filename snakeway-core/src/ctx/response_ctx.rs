use http::{Extensions, HeaderMap, StatusCode};

#[derive(Debug)]
pub struct ResponseCtx {
    pub request_id: Option<String>,
    pub status: StatusCode,
    pub headers: HeaderMap,
    #[allow(dead_code)]
    pub body: Vec<u8>,
    /// Request-scoped typed extensions propagated from RequestCtx.
    pub extensions: Extensions,
}

impl ResponseCtx {
    pub fn new(
        request_id: Option<String>,
        status: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
    ) -> Self {
        Self {
            request_id,
            status,
            headers,
            body,
            extensions: Extensions::new(),
        }
    }

    pub fn forbidden(request_id: Option<String>) -> Self {
        Self::new(
            request_id,
            StatusCode::FORBIDDEN,
            HeaderMap::new(),
            b"Forbidden".to_vec(),
        )
    }

    pub fn too_many_requests(request_id: Option<String>) -> Self {
        Self::new(
            request_id,
            StatusCode::TOO_MANY_REQUESTS,
            HeaderMap::new(),
            b"Too many requests".to_vec(),
        )
    }
}
