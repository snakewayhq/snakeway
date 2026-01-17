use http::{HeaderMap, StatusCode};

#[derive(Debug)]
pub struct ResponseCtx {
    pub request_id: Option<String>,
    pub status: StatusCode,
    pub headers: HeaderMap,
    #[allow(dead_code)]
    pub body: Vec<u8>,
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
        }
    }
}
