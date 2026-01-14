use crate::conf::types::*;
use crate::conf::validation::{ValidationReport, validate_ingresses, validate_redirect};
use pretty_assertions::assert_eq;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// Minimal valid service used to satisfy ingress validation
fn minimal_service() -> ServiceSpec {
    ServiceSpec {
        routes: vec![ServiceRouteSpec {
            path: "/".to_string(),
            ..Default::default()
        }],
        upstreams: vec![UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                port: 8080,
            }),
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn minimal_bind() -> BindSpec {
    BindSpec {
        interface: BindInterfaceInput::Keyword("loopback".to_string()),
        port: 8080,
        ..Default::default()
    }
}

fn minimal_admin_bind() -> BindAdminSpec {
    BindAdminSpec {
        interface: BindInterfaceInput::Keyword("loopback".to_string()),
        port: 9000,
        ..Default::default()
    }
}

pub fn minimal_ingress() -> IngressSpec {
    IngressSpec {
        bind: Some(minimal_bind()),
        services: vec![minimal_service()],
        ..Default::default()
    }
}

#[test]
fn validate_ingress_valid_minimal_bind() {
    // Arrange
    let mut report = ValidationReport::default();
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            ..Default::default()
        }),
        services: vec![minimal_service()],
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.has_violations(), false);
}

#[test]
fn validate_ingress_invalid_bind_addr() {
    // Arrange
    let mut report = ValidationReport::default();
    let addr = "not-an-addr".to_string();
    let expected_error = format!("invalid bind address: {}", addr);
    let ingress = minimal_ingress();

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert

    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

#[test]
fn validate_ingress_unspecified_ip_is_invalid() {
    // Arrange
    let mut report = ValidationReport::default();
    let addr = "0.0.0.0:8080".to_string();
    let expected_error = format!("invalid bind address: {}", addr);
    let ingress = minimal_ingress();

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert

    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

#[test]
fn validate_ingress_duplicate_bind_addr() {
    // Arrange
    let mut report = ValidationReport::default();
    let addr = "127.0.0.1:8080".to_string();
    let expected_error = format!("duplicate bind address: {}", addr);
    let ingress1 = minimal_ingress();
    let ingress2 = minimal_ingress();

    // Act
    validate_ingresses(&[ingress1, ingress2], &mut report);

    // Assert

    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

#[test]
fn validate_ingress_tls_missing_cert_and_key() {
    // Arrange
    let cert = PathBuf::from("/non/existent/cert.pem");
    let key = PathBuf::from("/non/existent/key.pem");
    let expected_cert_error = format!("missing cert file: {}", cert.display());
    let expected_key_error = format!("missing key file: {}", key.display());
    let mut report = ValidationReport::default();
    let mut bind = minimal_bind();
    bind.tls = Some(TlsSpec {
        cert: cert.to_string_lossy().to_string(),
        key: key.to_string_lossy().to_string(),
    });
    let ingress = IngressSpec {
        bind: Some(bind),
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_cert_error);
    assert_eq!(report.errors[1].message, expected_key_error);
}

#[test]
fn validate_ingress_http2_requires_tls() {
    // Arrange
    let mut report = ValidationReport::default();
    let mut bind = minimal_bind();

    let addr = bind.resolve().unwrap().to_string();

    let expected_error = format!("HTTP/2 requires TLS: {addr}");
    let expected_help = Some("Enable TLS on the bind or disable HTTP/2.".to_string());
    bind.enable_http2 = true;
    let ingress = IngressSpec {
        bind: Some(bind),
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors.first().unwrap().message, expected_error);
    assert_eq!(report.errors.first().unwrap().help, expected_help);
}

#[test]
fn validate_ingress_bind_admin_invalid_addr() {
    // Arrange
    let mut report = ValidationReport::default();
    let mut bind_admin = minimal_admin_bind();
    let addr = "bad-addr";
    bind_admin.interface = BindInterfaceInput::Keyword(addr.to_string());
    let expected_error = format!("invalid bind address: {}", addr);
    let ingress = IngressSpec {
        bind_admin: Some(bind_admin),
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

#[test]
fn validate_ingress_duplicate_admin_and_public_bind() {
    // Arrange
    let mut report = ValidationReport::default();
    let interface = BindInterfaceInput::Keyword("loopback".to_string());
    let port = 9000;
    let bind = BindSpec {
        interface: interface.clone(),
        port,
        ..Default::default()
    };
    let bind_admin = BindAdminSpec {
        interface: interface.clone(),
        port,
        ..Default::default()
    };
    let addr = format!("{}:{}", interface, port);
    let expected_error = format!("duplicate bind address: {}", addr);
    let ingress = IngressSpec {
        bind: Some(bind),
        bind_admin: Some(bind_admin),
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert

    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

fn test_origin() -> Origin {
    Origin::test("redirect_http_to_https")
}

#[test]
fn valid_3xx_status_produces_no_errors() {
    // Arrange
    let spec = RedirectSpec {
        port: 8080,
        status: 308,
    };
    let origin = test_origin();
    let mut report = ValidationReport::default();

    // Act
    validate_redirect(&spec, &origin, &mut report);

    // Assert
    assert_eq!(report.errors.is_empty(), true);
}

#[test]
fn valid_non_3xx_status_produces_error_bottom_of_range() {
    // Arrange
    let status = 299;
    let expected_error =
        format!("invalid redirect_response_code: {status} (must be between 300 and 399)");
    let spec = RedirectSpec { port: 8080, status };
    let origin = test_origin();
    let mut report = ValidationReport::default();

    // Act
    validate_redirect(&spec, &origin, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
}

#[test]
fn valid_non_3xx_status_produces_error_top_of_range() {
    // Arrange
    let status = 400;
    let expected_error =
        format!("invalid redirect_response_code: {status} (must be between 300 and 399)");
    let spec = RedirectSpec { port: 8080, status };
    let origin = test_origin();
    let mut report = ValidationReport::default();

    // Act
    validate_redirect(&spec, &origin, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
}

#[test]
fn invalid_port_produces_error() {
    // Arrange
    let spec = RedirectSpec {
        port: 0,
        status: 308,
    };
    let origin = test_origin();
    let mut report = ValidationReport::default();

    // Act
    validate_redirect(&spec, &origin, &mut report);

    // Assert
    assert_eq!(report.errors[0].message, "invalid port: 0");
}

#[test]
fn redirect_should_not_exist_without_tls() {
    // Arrange
    let addr = "127.0.0.1:8080".to_string();
    let expected_error = format!("redirect_http_to_https requires TLS: {addr}");
    let expected_help =
        Some("Enable TLS on the bind or remove redirect_http_to_https.".to_string());
    let mut report = ValidationReport::default();
    let mut bind = minimal_bind();
    bind.redirect_http_to_https = Some(RedirectSpec {
        port: 8080,
        status: 308,
    });
    let ingress = IngressSpec {
        bind: Some(bind),
        services: vec![minimal_service()],
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
    assert_eq!(report.errors[0].help, expected_help);
}
