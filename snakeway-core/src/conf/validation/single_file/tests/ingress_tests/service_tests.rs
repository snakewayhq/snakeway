use crate::conf::types::{
    BindInterfaceInput, BindSpec, CircuitBreakerConfig, EndpointSpec, HostSpec, IngressSpec,
    Origin, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use crate::conf::validation::{ValidationReport, validate_ingresses, validate_services};
use pretty_assertions::assert_eq;
use std::net::IpAddr;
use std::str::FromStr;

fn minimal_maybe_bind_addr() -> Option<BindSpec> {
    Some(BindSpec {
        interface: BindInterfaceInput::Keyword("loopback".to_string()),
        port: 8080,
        ..Default::default()
    })
}
fn minimal_upstream() -> UpstreamSpec {
    UpstreamSpec {
        endpoint: Some(EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 3000,
        }),
        weight: 1,
        ..Default::default()
    }
}

fn minimal_service() -> ServiceSpec {
    ServiceSpec {
        origin: Origin {
            section: "service_1".to_string(),
            ..Default::default()
        },
        upstreams: vec![minimal_upstream()],
        ..Default::default()
    }
}

#[test]
fn validate_multiple_services_at_once() {
    // Arrange
    let mut report = ValidationReport::default();

    let services = vec![
        minimal_service(),
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
        upstreams: vec![minimal_upstream()],
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
    let mut service = minimal_service();
    service.routes.push(ServiceRouteSpec {
        enable_websocket: true,
        ..Default::default()
    });
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
    let mut service = minimal_service();
    service.upstreams[0].weight = invalid_weight;
    let services = vec![service];
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
    let mut service = minimal_service();
    service.upstreams[0].weight = invalid_weight;
    let services = vec![service];
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
fn validate_service_upstream_cannot_have_both_endpoint_and_sock() {
    // Arrange
    let expected_error =
        "upstream cannot have both sock /tmp/test.sock and endpoint: 127.0.0.1:3000";
    let mut report = ValidationReport::default();
    let mut service = minimal_service();
    service.upstreams[0].endpoint = Some(EndpointSpec {
        host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
        port: 3000,
    });
    service.upstreams[0].sock = Some("/tmp/test.sock".to_string());
    let services = vec![service];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
}

#[test]
fn validate_service_upstream_must_have_either_addr_or_sock() {
    // Arrange
    let expected_error =
        "invalid upstream - it must have a sock or an endpoint, but neither are defined"
            .to_string();
    let mut report = ValidationReport::default();
    let mut upstream = minimal_upstream();
    upstream.endpoint = None;
    upstream.sock = None;
    let services = vec![ServiceSpec {
        upstreams: vec![upstream],
        ..Default::default()
    }];
    let maybe_bind = minimal_maybe_bind_addr();

    // Act
    validate_services(&maybe_bind, &services, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error)
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
        upstreams: vec![minimal_upstream()],
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
        upstreams: vec![minimal_upstream()],
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
        upstreams: vec![minimal_upstream()],
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
        upstreams: vec![minimal_upstream()],
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
        upstreams: vec![minimal_upstream()],
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

#[test]
fn validate_sock_file_not_reused_across_services() {
    // Arrange
    let sock = "/tmp/test.sock".to_string();
    let expected_error = format!("duplicate upstream sock: {}", sock);
    let mut report = ValidationReport::default();
    let services = vec![
        ServiceSpec {
            upstreams: vec![UpstreamSpec {
                sock: Some(sock.clone()),
                weight: 1,
                ..Default::default()
            }],
            ..Default::default()
        },
        ServiceSpec {
            upstreams: vec![UpstreamSpec {
                sock: Some(sock.clone()),
                weight: 1,
                ..Default::default()
            }],
            ..Default::default()
        },
    ];
    let bind = minimal_maybe_bind_addr();
    let ingresses = vec![IngressSpec {
        bind,
        services,
        ..Default::default()
    }];

    // Act
    validate_ingresses(&ingresses, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
}
