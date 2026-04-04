use http::HeaderMap;
use http::header::HeaderName;
use opentelemetry::propagation::{Extractor, Injector};
use pingora::http::RequestHeader;

/// Extracts W3C Trace Context headers (`traceparent`, `tracestate`) from an
/// incoming HTTP request's [`HeaderMap`].
pub(crate) struct HeaderExtractor<'a>(pub(crate) &'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key)?.to_str().ok()
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Injects W3C Trace Context headers (`traceparent`, `tracestate`) into an
/// upstream [`RequestHeader`].
pub(crate) struct RequestHeaderInjector<'a>(pub(crate) &'a mut RequestHeader);

impl Injector for RequestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            let _ = self.0.insert_header(name, &value);
        }
    }
}
