use crate::types::{ObservabilitySpec, Origin, OtelSpec};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReportDeprecated, range_constraint,
    validate_range_field,
};

range_constraint!(SAMPLING_RATIO, f64, min: 0.0, max: 1.0);

impl ValidateSpec for ObservabilitySpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        if let Some(otel) = &self.otel {
            otel.validate(origin, report);
        }
    }
}

impl ValidateSpec for OtelSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReportDeprecated) {
        if !self.enable {
            return;
        }

        if self.endpoint.is_empty() {
            report.otel_endpoint_cannot_be_empty(origin);
        } else if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            report.otel_endpoint_must_be_valid_url(origin);
        }

        if self.service_name.is_empty() {
            report.otel_service_name_cannot_be_empty(origin);
        }

        validate_range_field!(SAMPLING_RATIO, self.sampling_ratio, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{ObservabilitySpec, Origin, OtelSpec};
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};

    fn default_otel() -> OtelSpec {
        OtelSpec {
            enable: true,
            endpoint: "http://localhost:4317".to_string(),
            service_name: "snakeway".to_string(),
            sampling_ratio: 1.0,
        }
    }

    #[test]
    fn otel_disabled_skips_validation() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            enable: false,
            endpoint: String::new(),
            service_name: String::new(),
            sampling_ratio: 1.0,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn otel_endpoint_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            endpoint: String::new(),
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("endpoint") && e.message.contains("cannot be empty"))
        );
    }

    #[test]
    fn otel_endpoint_must_be_valid_url() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            endpoint: "not-a-url".to_string(),
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("endpoint") && e.message.contains("valid URL"))
        );
    }

    #[test]
    fn otel_service_name_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            service_name: String::new(),
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report.errors.iter().any(
                |e| e.message.contains("service_name") && e.message.contains("cannot be empty")
            )
        );
    }

    #[test]
    fn otel_valid_config() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = default_otel();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn observability_delegates_to_otel() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = ObservabilitySpec {
            otel: Some(OtelSpec {
                endpoint: String::new(),
                ..default_otel()
            }),
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
    }

    #[test]
    fn otel_sampling_ratio_below_zero() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            sampling_ratio: -0.1,
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("sampling_ratio"))
        );
    }

    #[test]
    fn otel_sampling_ratio_above_one() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            sampling_ratio: 1.1,
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("sampling_ratio"))
        );
    }

    #[test]
    fn otel_sampling_ratio_valid_fraction() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = OtelSpec {
            sampling_ratio: 0.5,
            ..default_otel()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn observability_without_otel_is_valid() {
        // Arrange
        let origin = Origin::test("observability");
        let mut report = ValidationReportDeprecated::default();
        let spec = ObservabilitySpec { otel: None };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }
}
