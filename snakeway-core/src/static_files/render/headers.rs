use crate::config::StaticCachePolicy;
use crate::static_files::render::range::ByteRange;
use http::{HeaderMap, HeaderName, HeaderValue, header};

#[derive(Debug, Default)]
pub(crate) struct HeaderBuilder {
    headers: HeaderMap,
}

impl HeaderBuilder {
    pub(crate) fn insert_header(&mut self, header_name: HeaderName, value: &str) {
        let header_value = HeaderValue::from_str(value).unwrap_or(HeaderValue::from_static(""));
        self.headers.insert(header_name, header_value);
    }

    pub(crate) fn accept_ranges(&mut self) {
        self.insert_header(header::ACCEPT_RANGES, "bytes");
    }

    pub(crate) fn content_type(&mut self, value: &str) {
        self.insert_header(header::CONTENT_TYPE, value);
    }

    pub(crate) fn content_length(&mut self, value: &str) {
        self.insert_header(header::CONTENT_LENGTH, value);
    }

    pub(crate) fn content_range(&mut self, range: ByteRange, len: u64) {
        self.insert_header(
            header::CONTENT_RANGE,
            &format!("bytes {}-{}/{}", range.start, range.end, len),
        );
    }

    pub(crate) fn content_encoding(&mut self, value: &str) {
        self.insert_header(header::CONTENT_ENCODING, value);
    }

    pub(crate) fn etag(&mut self, value: &str) {
        self.insert_header(header::ETAG, value);
    }

    pub(crate) fn last_modified(&mut self, value: &str) {
        self.insert_header(header::LAST_MODIFIED, value);
    }

    pub(crate) fn vary(&mut self) {
        self.insert_header(header::VARY, "Accept-Encoding");
    }

    pub(crate) fn cache_control(&mut self, policy: &StaticCachePolicy) {
        let mut value = String::new();

        if policy.public {
            value.push_str("public");
        } else {
            value.push_str("private");
        }

        value.push_str(&format!(", max-age={}", policy.max_age));

        if policy.immutable {
            value.push_str(", immutable");
        }

        self.insert_header(header::CACHE_CONTROL, &value);
    }

    pub(crate) fn build(self) -> HeaderMap {
        self.headers
    }
}
