use crate::conf::types::{
    BindSpec, CircuitBreakerConfig, Origin, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use crate::conf::validation::{ValidationReport, validate_services};

fn minimal_maybe_bind_addr() -> Option<BindSpec> {
    Some(BindSpec {
        addr: "127.0.0.1:8080".to_string(),
        ..BindSpec::default()
    })
}

#[test]
fn validate_multiple_services_at_once() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![
        ServiceSpec {
            origin: Origin {
                section: "service_1".to_string(),
                ..Default::default()
            },
            upstreams: vec![UpstreamSpec {
                addr: Some("127.0.0.1:8080".to_string()),
                weight: 1,
                ..Default::default()
            }],
            ..Default::default()
        },
        ServiceSpec {
            origin: Origin {
                section: "service_2".to_string(),
                ..Default::default()
            },
            upstreams: vec![], // Invalid: no upstreams
            ..Default::default()
        },
    ];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    assert!(
        report
            .errors
            .iter()
            .any(|w| { w.message.contains("service has no upstream backends") })
    )
}

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
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

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
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

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
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains(expected));
    assert!(report.has_violations());
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
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains(&expected));
    assert!(report.has_violations());
    assert!(report.warnings.is_empty());
}

#[test]
fn validate_service_must_have_an_upstream_with_weight_not_greater_than_1000() {
    // Arrange
    let invalid_weight = 1001;
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
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains(&expected));
    assert!(report.has_violations());
    assert!(report.warnings.is_empty());
}

#[test]
fn validate_service_upstream_cannot_have_both_addr_and_sock() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            sock: Some("/tmp/test.sock".to_string()),
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("mutually exclusive"));
}

#[test]
fn validate_service_upstream_must_have_either_addr_or_sock() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: None,
            sock: None,
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("mutually exclusive"));
}

#[test]
fn validate_service_upstream_with_invalid_addr() {
    // Arrange
    let invalid_addr = "not-an-ip";
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some(invalid_addr.to_string()),
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("invalid upstream address"));
}

#[test]
fn validate_service_duplicate_upstream_socks() {
    // Arrange
    let duplicate_sock = "/tmp/test.sock".to_string();
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![
            UpstreamSpec {
                sock: Some(duplicate_sock.clone()),
                weight: 1,
                ..Default::default()
            },
            UpstreamSpec {
                sock: Some(duplicate_sock.clone()),
                weight: 1,
                ..Default::default()
            },
        ],
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("duplicate upstream sock"));
}

#[test]
fn validate_service_circuit_breaker_valid() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        circuit_breaker: Some(CircuitBreakerConfig {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        }),
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    assert!(!report.has_violations());
}

#[test]
fn validate_service_circuit_breaker_failure_threshold_out_of_range() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        circuit_breaker: Some(CircuitBreakerConfig {
            enable_auto_recovery: true,
            failure_threshold: 0, // Min is 1
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        }),
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("circuit_breaker.failure_threshold"));
}

#[test]
fn validate_service_circuit_breaker_open_duration_out_of_range() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        circuit_breaker: Some(CircuitBreakerConfig {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 0, // Min is 1
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        }),
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(
        error
            .message
            .contains("circuit_breaker.open_duration_milliseconds")
    );
}

#[test]
fn validate_service_circuit_breaker_half_open_max_requests_out_of_range() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        circuit_breaker: Some(CircuitBreakerConfig {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 10001, // Max is 10_000
            success_threshold: 2,
            ..Default::default()
        }),
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(
        error
            .message
            .contains("circuit_breaker.half_open_max_requests")
    );
}

#[test]
fn validate_service_circuit_breaker_success_threshold_out_of_range() {
    // Arrange
    let mut report = ValidationReport::default();
    let services = vec![ServiceSpec {
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        circuit_breaker: Some(CircuitBreakerConfig {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 0, // Min is 1
            ..Default::default()
        }),
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    let error = report.errors.first().expect("expected at least one error");
    assert!(error.message.contains("circuit_breaker.success_threshold"));
}
