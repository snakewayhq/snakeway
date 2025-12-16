use http::{HeaderMap, StatusCode};

#[derive(Debug)]
pub struct ResponseCtx {
    pub status: StatusCode,
    pub headers: HeaderMap,
    #[allow(dead_code)]
    pub body: Vec<u8>,
}

impl ResponseCtx {
    pub fn new(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }
}
