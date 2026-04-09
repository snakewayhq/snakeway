use crate::types::{Origin, RequestFilterDeviceSpec};
use crate::validation::validator::{
    validate_device_paths, validate_http_header_name, validate_http_method,
};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(DENY_STATUS, u16, min: 400, max: 599);

impl ValidateSpec for RequestFilterDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if let Some(deny_status) = self.deny_status {
            validate_range_field!(DENY_STATUS, deny_status, report, origin);
        }

        for method in &self.allow_methods {
            validate_http_method(method, report, origin);
        }

        for method in &self.deny_methods {
            validate_http_method(method, report, origin);
        }

        for header in &self.deny_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.allow_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.required_headers {
            validate_http_header_name(header, report, origin);
        }

        validate_device_paths(&self.paths, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::RequestFilterDeviceSpec;
    use crate::validation::{ValidateSpec, ValidationReport};

    #[test]
    fn deny_status_below_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestFilterDeviceSpec {
            enable: true,
            deny_status: Some(399),
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("deny_status"))
        );
    }

    #[test]
    fn deny_status_above_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestFilterDeviceSpec {
            enable: true,
            deny_status: Some(600),
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("deny_status"))
        );
    }

    #[test]
    fn invalid_http_method_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestFilterDeviceSpec {
            enable: true,
            allow_methods: vec!["INVALID METHOD".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("invalid HTTP method"))
        );
    }

    #[test]
    fn valid_request_filter() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestFilterDeviceSpec {
            enable: true,
            deny_status: Some(403),
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn path_without_leading_slash_is_invalid() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestFilterDeviceSpec {
            enable: true,
            paths: vec!["api/v1".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("must start with '/'"))
        );
    }
}
