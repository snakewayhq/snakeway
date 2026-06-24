use http::{HeaderMap, Method, Uri, Version};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub(crate) struct NormalizedRequestParams {
    pub(crate) original_uri: Uri,
    pub(crate) method: Method,
    pub(crate) host: String,
    pub(crate) path: NormalizedPath,
    #[allow(dead_code)] // reserved for future routing / logging / caching
    pub(crate) query: CanonicalQuery,
    pub(crate) headers: NormalizedHeaders,
    pub(crate) sni_host: Option<String>,
    pub(crate) protocol_version: Version,
    pub(crate) is_upgrade_req: bool,
}

#[derive(Debug, Default)]
pub(crate) struct NormalizedRequest(NormalizedRequestParams);

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
    pub(crate) fn original_uri(&self) -> &Uri {
        &self.original_uri
    }

    pub(crate) fn effective_host(&self) -> &str {
        self.sni_host.as_deref().unwrap_or(&self.host)
    }

    pub(crate) fn method(&self) -> &Method {
        &self.method
    }

    pub(crate) fn path(&self) -> &NormalizedPath {
        &self.path
    }

    #[allow(dead_code)] // reserved for future routing / logging / caching
    pub(crate) fn query(&self) -> &CanonicalQuery {
        &self.query
    }

    pub(crate) fn headers(&self) -> &HeaderMap {
        &self.headers.header_map
    }

    pub(crate) fn is_upgrade_req(&self) -> bool {
        self.is_upgrade_req
    }

    pub(crate) fn is_http2(&self) -> bool {
        self.protocol_version == Version::HTTP_2
    }
}

/// WASM Device API
impl NormalizedRequest {
    pub(crate) fn set_path(&mut self, path: String) {
        self.path.0 = path;
    }
    pub(crate) fn insert_header(
        &mut self,
        name: http::header::HeaderName,
        value: http::header::HeaderValue,
    ) {
        self.headers.header_map.insert(name, value);
    }

    pub(crate) fn append_header(
        &mut self,
        name: http::header::HeaderName,
        value: http::header::HeaderValue,
    ) {
        self.headers.header_map.append(name, value);
    }

    pub(crate) fn remove_header(&mut self, name: &str) {
        self.headers.header_map.remove(name);
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
pub(crate) struct NormalizedPath(pub(crate) String);

impl NormalizedPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Default)]
pub(crate) struct CanonicalQuery {
    #[allow(dead_code)] // reserved for future routing / logging / caching
    raw: String,
    #[allow(dead_code)] // reserved for future routing / logging / caching
    pairs: SmallVec<[(String, String); 4]>,
}

impl CanonicalQuery {
    pub(crate) fn new(raw: &str, pairs: SmallVec<[(String, String); 4]>) -> Self {
        Self {
            raw: raw.to_string(),
            pairs,
        }
    }

    #[allow(dead_code)] // reserved for future routing / logging / caching
    pub(crate) fn raw(&self) -> &str {
        &self.raw
    }

    #[allow(dead_code)] // reserved for future routing / logging / caching
    pub(crate) fn pairs(&self) -> &[(String, String)] {
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
    pub(crate) fn new(headers: HeaderMap) -> Self {
        Self {
            header_map: headers,
        }
    }

    pub(crate) fn as_map(&self) -> &HeaderMap {
        &self.header_map
    }
}
