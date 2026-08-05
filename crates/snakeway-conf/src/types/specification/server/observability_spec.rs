use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(SAMPLING_RATIO, f64, min: 0.0, max: 1.0);

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct ObservabilitySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub otel: Option<Located<OtelSpec>>,
}

impl Validate for ObservabilitySpec {
    fn validate(&self, _report: &mut Report) {}
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct OtelSpec {
    pub enable: Located<bool>,
    pub endpoint: Located<String>,
    pub service_name: Located<String>,
    #[confval(default = 1.0)]
    pub sampling_ratio: Located<f64>,
}

impl Validate for OtelSpec {
    fn validate(&self, report: &mut Report) {
        // Requirement checks: an enabled exporter needs an endpoint and a service name.
        if self.enable.value {
            if self.endpoint.value.is_empty() {
                report
                    .error("observability.otel.endpoint cannot be empty when enabled")
                    .at(self.endpoint.span)
                    .help("Provide the gRPC endpoint for the OTLP exporter (e.g., http://localhost:4317).")
                    .emit();
            }

            if self.service_name.value.is_empty() {
                report
                    .error("observability.otel.service_name cannot be empty when enabled")
                    .at(self.service_name.span)
                    .emit();
            }
        }

        if !self.endpoint.value.is_empty()
            && !self.endpoint.value.starts_with("http://")
            && !self.endpoint.value.starts_with("https://")
        {
            report
                .error("observability.otel.endpoint must be a valid URL")
                .at(self.endpoint.span)
                .help("The endpoint must start with http:// or https://.")
                .emit();
        }

        SAMPLING_RATIO.check_located(&self.sampling_ratio, "sampling_ratio", report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::SourceMap;

    fn default_otel() -> OtelSpec {
        OtelSpec {
            enable: Located::detached(true),
            endpoint: Located::detached("http://localhost:4317".to_string()),
            service_name: Located::detached("snakeway".to_string()),
            sampling_ratio: Located::detached(1.0),
        }
    }

    #[test]
    fn parse_observability_with_otel_block() {
        // Arrange
        let input = r#"otel {
  enable = true
  endpoint = "http://localhost:4317"
  service_name = "snakeway"
  sampling_ratio = 0.5
}
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("snakeway.hcl", input);

        // Act
        let spec = parse_hcl::<ObservabilitySpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let otel = spec.unwrap().otel.unwrap();
        assert!(otel.value.enable.value);
        assert_eq!(otel.value.sampling_ratio.value, 0.5);
    }

    #[test]
    fn parse_otel_sampling_ratio_defaults_to_one() {
        // Arrange
        let input = r#"otel {
  enable = true
  endpoint = "http://localhost:4317"
  service_name = "snakeway"
}
"#;
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("snakeway.hcl", input);

        // Act
        let spec = parse_hcl::<ObservabilitySpec>(&sources, id, &mut report);

        // Assert
        assert!(!report.has_issues());
        let otel = spec.unwrap().otel.unwrap();
        assert_eq!(otel.value.sampling_ratio.value, 1.0);
    }

    #[test]
    fn otel_disabled_still_validates_present_values() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            enable: Located::detached(false),
            endpoint: Located::detached("ftp://collector".to_string()),
            service_name: Located::detached("snakeway".to_string()),
            sampling_ratio: Located::detached(2.0),
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("endpoint")),
            "a disabled otel block must still validate a present endpoint; issues: {:?}",
            report.issues()
        );
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("sampling_ratio")),
            "a disabled otel block must still validate sampling_ratio; issues: {:?}",
            report.issues()
        );
    }

    /// The empty checks are requirement checks tied to the feature being on,
    /// so a disabled block with unset fields stays valid.
    #[test]
    fn otel_disabled_allows_empty_endpoint_and_service_name() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            enable: Located::detached(false),
            endpoint: Located::detached(String::new()),
            service_name: Located::detached(String::new()),
            sampling_ratio: Located::detached(1.0),
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn otel_endpoint_cannot_be_empty() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            endpoint: Located::detached(String::new()),
            ..default_otel()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "observability.otel.endpoint cannot be empty when enabled")
        );
    }

    #[test]
    fn otel_endpoint_must_be_valid_url() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            endpoint: Located::detached("localhost:4317".to_string()),
            ..default_otel()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "observability.otel.endpoint must be a valid URL")
        );
    }

    #[test]
    fn otel_service_name_cannot_be_empty() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            service_name: Located::detached(String::new()),
            ..default_otel()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.issues().iter().any(
                |i| i.message == "observability.otel.service_name cannot be empty when enabled"
            )
        );
    }

    #[test]
    fn otel_sampling_ratio_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            sampling_ratio: Located::detached(1.5),
            ..default_otel()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "sampling_ratio must be at most 1")
        );
    }
}
