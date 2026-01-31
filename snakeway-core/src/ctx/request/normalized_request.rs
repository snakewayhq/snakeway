use http::{HeaderMap, Method, Uri, Version};

#[derive(Debug, Default)]
pub struct NormalizedRequest {
    original_uri: Uri,
    method: Method,
    path: NormalizedPath,
    query: CanonicalQuery,
    normalized_headers: NormalizedHeaders,
    protocol_version: Version,
    is_upgrade_req: bool,
}

impl NormalizedRequest {
    pub fn new(
        original_uri: Uri,
        method: Method,
        path: NormalizedPath,
        query: CanonicalQuery,
        headers: NormalizedHeaders,
        protocol_version: Version,
        is_upgrade_req: bool,
    ) -> Self {
        Self {
            original_uri,
            method,
            path,
            query,
            normalized_headers: headers,
            protocol_version,
            is_upgrade_req,
        }
    }

    pub fn original_uri(&self) -> &Uri {
        &self.original_uri
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &NormalizedPath {
        &self.path
    }

    pub fn set_path(&mut self, path: String) {
        self.path.0 = path;
    }

    pub fn query(&self) -> &CanonicalQuery {
        &self.query
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.normalized_headers.header_map
    }

    pub fn insert_header(
        &mut self,
        name: http::header::HeaderName,
        value: http::header::HeaderValue,
    ) {
        self.normalized_headers.header_map.insert(name, value);
    }

    pub fn remove_header(&mut self, name: &str) {
        self.normalized_headers.header_map.remove(name);
    }

    pub fn is_upgrade_req(&self) -> bool {
        self.is_upgrade_req
    }

    pub fn protocol_version(&self) -> &Version {
        &self.protocol_version
    }

    pub fn is_http2(&self) -> bool {
        self.protocol_version == Version::HTTP_2
    }

    pub fn into_inner(self) -> (Method, NormalizedPath, CanonicalQuery, NormalizedHeaders) {
        (self.method, self.path, self.query, self.normalized_headers)
    }
}

/// Used for testing purposes only.
impl From<NormalizedPath> for NormalizedRequest {
    fn from(path: NormalizedPath) -> Self {
        NormalizedRequest {
            path,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct NormalizedPath(pub String);

impl NormalizedPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct CanonicalQuery {
    raw: String,
    pairs: Vec<(String, String)>,
}

impl CanonicalQuery {
    pub fn new(raw: &str, pairs: Vec<(String, String)>) -> Self {
        Self {
            raw: raw.to_string(),
            pairs,
        }
    }

    pub fn from_raw(raw: Option<&str>) -> CanonicalQuery {
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

#[derive(Debug, Default)]
pub struct NormalizedHeaders {
    header_map: HeaderMap,
}

impl From<HeaderMap> for NormalizedHeaders {
    fn from(headers: HeaderMap) -> Self {
        NormalizedHeaders {
            header_map: headers,
        }
    }
}

impl NormalizedHeaders {
    pub fn new(headers: HeaderMap) -> Self {
        Self {
            header_map: headers,
        }
    }

    pub fn as_map(&self) -> &HeaderMap {
        &self.header_map
    }
}
