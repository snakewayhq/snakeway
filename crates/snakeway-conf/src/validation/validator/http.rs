use confval::prelude::Format;
use http::{HeaderName, Method};

/// An HTTP method name, such as `GET` or `POST`.
pub(crate) struct HttpMethod;

impl Format for HttpMethod {
    const NAME: &'static str = "HTTP method";

    fn check(value: &str) -> bool {
        Method::from_bytes(value.as_bytes()).is_ok()
    }
}

/// An HTTP header name, such as `content-type`.
pub(crate) struct HttpHeaderName;

impl Format for HttpHeaderName {
    const NAME: &'static str = "HTTP header name";

    fn check(value: &str) -> bool {
        HeaderName::from_bytes(value.as_bytes()).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::Format;

    #[test]
    fn valid_header_name_passes() {
        // Arrange
        let value = "content-type";

        // Act
        let result = HttpHeaderName::check(value);

        // Assert
        assert!(result);
    }

    #[test]
    fn invalid_header_name_fails() {
        // Arrange
        let value = "invalid header!";

        // Act
        let result = HttpHeaderName::check(value);

        // Assert
        assert!(!result);
    }

    #[test]
    fn valid_http_method_passes() {
        // Arrange
        let value = "GET";

        // Act
        let result = HttpMethod::check(value);

        // Assert
        assert!(result);
    }

    #[test]
    fn invalid_http_method_fails() {
        // Arrange
        let value = "INVALID METHOD";

        // Act
        let result = HttpMethod::check(value);

        // Assert
        assert!(!result);
    }
}
