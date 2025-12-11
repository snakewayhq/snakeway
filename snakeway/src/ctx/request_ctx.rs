use http::{HeaderMap, Method, Uri};

pub struct RequestCtx {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl RequestCtx {
    pub fn new(method: Method, uri: Uri, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
        }
    }
}
