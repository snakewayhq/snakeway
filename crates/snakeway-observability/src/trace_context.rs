use http::HeaderMap;
use http::header::HeaderName;
use opentelemetry::propagation::{Extractor, Injector};
use pingora_http::RequestHeader;

/// Extracts W3C Trace Context headers (`traceparent`, `tracestate`) from an
/// incoming HTTP request's [`HeaderMap`].
pub struct HeaderExtractor<'a>(pub &'a HeaderMap);

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
pub struct RequestHeaderInjector<'a>(pub &'a mut RequestHeader);

impl Injector for RequestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            let _ = self.0.insert_header(name, &value);
        }
    }
}

#[cfg(test)]
mod header_extractor_tests {
    use super::*;
    use opentelemetry::propagation::Extractor;

    #[test]
    fn extractor_returns_header_value() {
        // Arrange
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01"
                .parse()
                .unwrap(),
        );

        // Act
        let extractor = HeaderExtractor(&headers);
        let value = extractor.get("traceparent");

        // Assert
        assert_eq!(
            value,
            Some("00-abcdef1234567890abcdef1234567890-1234567890abcdef-01")
        );
    }

    #[test]
    fn extractor_returns_none_for_missing_key() {
        // Arrange
        let headers = HeaderMap::new();

        // Act
        let extractor = HeaderExtractor(&headers);
        let value = extractor.get("traceparent");

        // Assert
        assert_eq!(value, None);
    }

    #[test]
    fn extractor_keys_returns_all_header_names() {
        // Arrange
        let mut headers = HeaderMap::new();
        headers.insert("traceparent", "value1".parse().unwrap());
        headers.insert("tracestate", "value2".parse().unwrap());
        headers.insert("host", "example.com".parse().unwrap());

        // Act
        let extractor = HeaderExtractor(&headers);
        let mut keys = extractor.keys();
        keys.sort();

        // Assert
        assert_eq!(keys, vec!["host", "traceparent", "tracestate"]);
    }
}

#[cfg(test)]
mod request_header_injector_tests {
    use super::*;
    use opentelemetry::propagation::Injector;

    #[test]
    fn injector_sets_header_on_request() {
        // Arrange
        let mut req = RequestHeader::build("GET", b"/", None).unwrap();
        let header_value = "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01";

        // Act
        let mut injector = RequestHeaderInjector(&mut req);
        injector.set("traceparent", header_value.to_string());

        // Assert
        let value = req.headers.get("traceparent").unwrap().to_str().unwrap();
        assert_eq!(value, header_value);
    }

    #[test]
    fn injector_ignores_invalid_header_name() {
        // Arrange
        let mut req = RequestHeader::build("GET", b"/", None).unwrap();
        let header_count_before = req.headers.len();
        let header_value = "some-value".to_string();

        // Act
        let mut injector = RequestHeaderInjector(&mut req);
        injector.set("", header_value);

        // Assert
        assert_eq!(req.headers.len(), header_count_before);
    }
}
