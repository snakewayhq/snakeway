use http::{HeaderMap, Method, Uri, Version};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct NormalizedRequestParams {
    pub original_uri: Uri,
    pub method: Method,
    pub host: String,
    pub path: NormalizedPath,
    pub query: CanonicalQuery,
    pub headers: NormalizedHeaders,
    pub sni_host: Option<String>,
    pub protocol_version: Version,
    pub is_upgrade_req: bool,
}

#[derive(Debug, Default)]
pub struct NormalizedRequest(NormalizedRequestParams);

impl From<NormalizedRequestParams> for NormalizedRequest {
    fn from(params: NormalizedRequestParams) -> Self {
        Self(params)
    }
}

impl Deref for NormalizedRequest {
    type Target = NormalizedRequestParams;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NormalizedRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NormalizedRequest {
    pub fn original_uri(&self) -> &Uri {
        &self.original_uri
    }

    pub fn effective_host(&self) -> &str {
        self.sni_host.as_deref().unwrap_or(&self.host)
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
        &self.headers.header_map
    }

    pub fn insert_header(
        &mut self,
        name: http::header::HeaderName,
        value: http::header::HeaderValue,
    ) {
        self.headers.header_map.insert(name, value);
    }

    pub fn remove_header(&mut self, name: &str) {
        self.headers.header_map.remove(name);
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
        let NormalizedRequestParams {
            method,
            path,
            query,
            headers,
            ..
        } = self.0;

        (method, path, query, headers)
    }
}

/// Used for testing purposes only.
impl From<NormalizedPath> for NormalizedRequest {
    fn from(path: NormalizedPath) -> Self {
        NormalizedRequestParams {
            path,
            ..Default::default()
        }
        .into()
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
