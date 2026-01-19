use http::{HeaderMap, Method};

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

impl NormalizedPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct CanonicalQuery {
    raw: String,
    /// todo actually use this.
    pairs: Vec<(String, String)>,
}

impl CanonicalQuery {
    pub(crate) fn from_raw(raw: Option<&str>) -> CanonicalQuery {
        let raw = raw.unwrap_or("").to_string();

        CanonicalQuery {
            raw,
            pairs: Vec::new(),
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }
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
pub struct NormalizedHeaders {
    /// todo actually use this.
    headers: HeaderMap,
}

impl From<HeaderMap> for NormalizedHeaders {
    fn from(headers: HeaderMap) -> Self {
        NormalizedHeaders { headers }
    }
}
