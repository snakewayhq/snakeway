use http::Method;
use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct RequestId(pub String);

impl Default for RequestId {
    fn default() -> Self {
        RequestId(Uuid::new_v4().to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId(s.to_owned())
    }
}

#[derive(Debug)]
pub struct NormalizedRequest {
    method: Method,
    path: NormalizedPath,
    query: CanonicalQuery,
    headers: NormalizedHeaders,
}

impl NormalizedRequest {
    pub fn new(
        method: Method,
        path: NormalizedPath,
        query: CanonicalQuery,
        headers: NormalizedHeaders,
    ) -> Self {
        Self {
            method,
            path,
            query,
            headers,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &NormalizedPath {
        &self.path
    }

    pub fn query(&self) -> &CanonicalQuery {
        &self.query
    }

    pub fn headers(&self) -> &NormalizedHeaders {
        &self.headers
    }

    pub fn into_inner(self) -> (Method, NormalizedPath, CanonicalQuery, NormalizedHeaders) {
        (self.method, self.path, self.query, self.headers)
    }
}

#[derive(Debug)]
pub struct NormalizedPath(pub String);

#[derive(Debug)]
pub struct CanonicalQuery(pub String);

#[derive(Debug)]
pub struct NormalizedHeaders(pub http::HeaderMap);
