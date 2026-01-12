use crate::conf::types::{ServiceRouteSpec, ServiceSpec, UpstreamSpec};
use crate::conf::validation::{ValidationReport, validate_services};

#[test]
fn validate_minimum_service_spec() {
    // Arrange
    let mut report = ValidationReport::default();
    let service = ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    };
    let services = vec![service];

    // Act
    validate_services(&services, &mut report);

    // Assert
    assert!(!report.has_violations());
    assert!(report.errors.is_empty());
    assert!(report.warnings.is_empty());
}

#[test]
fn validate_websocket_service() {
    // Arrange
    let mut report = ValidationReport::default();
    let service = ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        routes: vec![ServiceRouteSpec {
            enable_websocket: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    let services = vec![service];

    // Act
    validate_services(&services, &mut report);

    // Assert
    assert!(!report.has_violations());
    assert!(report.errors.is_empty());
    assert!(report.warnings.is_empty());
}

#[test]
fn validate_service_but_have_an_upstream() {
    // Arrange
    let expected = "service has no upstream backends";
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![],
        ..Default::default()
    }];

    // Act
    validate_services(&services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains(expected));
    assert!(report.has_violations());
    assert_eq!(report.errors.len(), 1);
    assert!(report.warnings.is_empty());
}

#[test]
fn validate_service_must_have_an_upstream_with_weight_greater_than_zero() {
    // Arrange
    let invalid_weight = 0;
    let expected = format!("invalid upstream weight: {}", invalid_weight);
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: invalid_weight,
            ..Default::default()
        }],
        ..Default::default()
    }];

    // Act
    validate_services(&services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains(&expected));
    assert_eq!(report.errors.len(), 1);
    assert!(report.has_violations());
    assert!(report.warnings.is_empty());
}
