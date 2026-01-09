use crate::conf::types::StaticCachePolicy;
use crate::static_files::render::range::ByteRange;
use http::{HeaderMap, HeaderName, HeaderValue, header};

#[derive(Debug, Default)]
pub(crate) struct HeaderBuilder {
    headers: HeaderMap,
}

impl HeaderBuilder {
    /// Inserts (or removes) a header from the header map.
    ///
    /// Converts the provided string value into a `HeaderValue`. If the conversion fails
    /// or results in an empty value, the header is removed from the map. Otherwise,
    /// the header is inserted with the given name and value.
    ///
    /// # Arguments
    ///
    /// * `header_name` - The name of the HTTP header to insert or remove
    /// * `value` - The string value to convert and insert as the header value
    pub(crate) fn insert(&mut self, header_name: HeaderName, value: &str) {
        let header_value = HeaderValue::from_str(value).unwrap_or(HeaderValue::from_static(""));
        if header_value.is_empty() {
            self.headers.remove(header_name);
        } else {
            self.headers.insert(header_name, header_value);
        }
    }

    pub(crate) fn accept_ranges(&mut self) {
        self.insert(header::ACCEPT_RANGES, "bytes");
    }

    pub(crate) fn content_type(&mut self, value: &str) {
        self.insert(header::CONTENT_TYPE, value);
    }

    pub(crate) fn content_length(&mut self, value: &str) {
        self.insert(header::CONTENT_LENGTH, value);
    }

    pub(crate) fn content_range(&mut self, range: ByteRange, len: u64) {
        self.insert(
            header::CONTENT_RANGE,
            &format!("bytes {}-{}/{}", range.start, range.end, len),
        );
    }

    pub(crate) fn content_encoding(&mut self, value: &str) {
        self.insert(header::CONTENT_ENCODING, value);
    }

    pub(crate) fn etag(&mut self, value: &str) {
        self.insert(header::ETAG, value);
    }

    pub(crate) fn last_modified(&mut self, value: &str) {
        self.insert(header::LAST_MODIFIED, value);
    }

    pub(crate) fn vary(&mut self) {
        self.insert(header::VARY, "Accept-Encoding");
    }

    pub(crate) fn cache_control(&mut self, policy: &StaticCachePolicy) {
        let mut value = String::new();

        if policy.public {
            value.push_str("public");
        } else {
            value.push_str("private");
        }

        value.push_str(&format!(", max-age={}", policy.max_age_seconds));

        if policy.immutable {
            value.push_str(", immutable");
        }

        self.insert(header::CACHE_CONTROL, &value);
    }

    pub(crate) fn build(self) -> HeaderMap {
        self.headers
    }
}
