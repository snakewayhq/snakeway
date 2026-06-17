use confval::prelude::{Located, Report};
use http::{HeaderName, Method};

pub(crate) fn validate_http_header_name(header: &Located<String>, report: &mut Report) {
    if HeaderName::from_bytes(header.value.as_bytes()).is_err() {
        report
            .error(format!("invalid HTTP header name: {}", header.value))
            .at(header.span)
            .emit();
    }
}

pub(crate) fn validate_http_method(method: &Located<String>, report: &mut Report) {
    if Method::from_bytes(method.value.as_bytes()).is_err() {
        report
            .error(format!("invalid HTTP method: {}", method.value))
            .at(method.span)
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn located(value: &str) -> Located<String> {
        Located::detached(value.to_string())
    }

    #[test]
    fn valid_header_name() {
        // Arrange
        let mut report = Report::new();

        // Act
        validate_http_header_name(&located("content-type"), &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn invalid_header_name() {
        // Arrange
        let mut report = Report::new();

        // Act
        validate_http_header_name(&located("invalid header!"), &mut report);

        // Assert
        assert!(report.has_issues());
        assert_eq!(report.issues().len(), 1);
        assert!(
            report.issues()[0]
                .message
                .contains("invalid HTTP header name"),
            "expected error to contain 'invalid HTTP header name', got: {}",
            report.issues()[0].message
        );
    }

    #[test]
    fn valid_http_method() {
        // Arrange
        let mut report = Report::new();

        // Act
        validate_http_method(&located("GET"), &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn invalid_http_method() {
        // Arrange
        let mut report = Report::new();

        // Act
        validate_http_method(&located("INVALID METHOD"), &mut report);

        // Assert
        assert!(report.has_issues());
        assert_eq!(report.issues().len(), 1);
        assert!(
            report.issues()[0].message.contains("invalid HTTP method"),
            "expected error to contain 'invalid HTTP method', got: {}",
            report.issues()[0].message
        );
    }
}
