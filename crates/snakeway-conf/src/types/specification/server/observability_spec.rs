use confval::provenance::{Located, Report};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(SAMPLING_RATIO, f64, min: 0.0, max: 1.0);

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct ObservabilitySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub otel: Option<Located<OtelSpec>>,
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct OtelSpec {
    pub enable: Located<bool>,
    pub endpoint: Located<String>,
    pub service_name: Located<String>,
    #[confval(default = 1.0)]
    pub sampling_ratio: Located<f64>,
}

pub fn validate_observability(spec: &ObservabilitySpec, report: &mut Report) {
    if let Some(otel) = &spec.otel {
        validate_otel(&otel.value, report);
    }
}

fn validate_otel(spec: &OtelSpec, report: &mut Report) {
    if !spec.enable.value {
        return;
    }

    if spec.endpoint.value.is_empty() {
        report
            .error("observability.otel.endpoint cannot be empty when enabled")
            .at(spec.endpoint.span)
            .help("Provide the gRPC endpoint for the OTLP exporter (e.g., http://localhost:4317).")
            .emit();
    } else if !spec.endpoint.value.starts_with("http://")
        && !spec.endpoint.value.starts_with("https://")
    {
        report
            .error("observability.otel.endpoint must be a valid URL")
            .at(spec.endpoint.span)
            .help("The endpoint must start with http:// or https://.")
            .emit();
    }

    if spec.service_name.value.is_empty() {
        report
            .error("observability.otel.service_name cannot be empty when enabled")
            .at(spec.service_name.span)
            .emit();
    }

    SAMPLING_RATIO.check_located(&spec.sampling_ratio, "sampling_ratio", report);
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::hcl::parse_hcl;
    use confval::provenance::SourceMap;

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
    fn otel_disabled_skips_validation() {
        // Arrange
        let mut report = Report::new();
        let spec = OtelSpec {
            enable: Located::detached(false),
            endpoint: Located::detached(String::new()),
            service_name: Located::detached(String::new()),
            sampling_ratio: Located::detached(1.0),
        };

        // Act
        validate_otel(&spec, &mut report);

        // Assert
        assert!(!report.has_issues());
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
        validate_otel(&spec, &mut report);

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
        validate_otel(&spec, &mut report);

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
        validate_otel(&spec, &mut report);

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
        validate_otel(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "sampling_ratio must be at most 1")
        );
    }
}
