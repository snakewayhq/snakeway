use http::Method;

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
pub struct CanonicalQuery {
    raw: String,
    pairs: Vec<(String, String)>,
}

impl CanonicalQuery {
    pub fn new(raw: &str) -> Self {
        Self {
            raw: raw.to_string(),
            pairs: vec![],
        }
    }
}

#[derive(Debug)]
pub struct NormalizedHeaders;
