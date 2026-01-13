use crate::conf::types::*;
use crate::conf::validation::{ValidationReport, validate_ingresses, validate_redirect};
use pretty_assertions::assert_eq;
use std::path::PathBuf;

/// Minimal valid service used to satisfy ingress validation
fn minimal_service() -> ServiceSpec {
    ServiceSpec {
        routes: vec![ServiceRouteSpec {
            path: "/".to_string(),
            ..Default::default()
        }],
        upstreams: vec![UpstreamSpec {
            addr: Some("127.0.0.1:8080".to_string()),
            weight: 1,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn validate_ingress_valid_minimal_bind() {
    // Arrange
    let mut report = ValidationReport::default();
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr: "127.0.0.1:8080".to_string(),
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
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr,
            ..Default::default()
        }),
        ..Default::default()
    };

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
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr,
            ..Default::default()
        }),
        ..Default::default()
    };

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

    let ingress1 = IngressSpec {
        bind: Some(BindSpec {
            addr: addr.clone(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let ingress2 = IngressSpec {
        bind: Some(BindSpec {
            addr,
            ..Default::default()
        }),
        ..Default::default()
    };

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
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr: "127.0.0.1:8443".to_string(),
            tls: Some(TlsSpec {
                cert: cert.to_string_lossy().to_string(),
                key: key.to_string_lossy().to_string(),
            }),
            ..Default::default()
        }),
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
    let addr = "127.0.0.1:8080".to_string();
    let expected_error = format!("HTTP/2 requires TLS: {addr}");
    let expected_help = Some("Enable TLS on the bind or disable HTTP/2.".to_string());
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr,
            enable_http2: true,
            tls: None,
            ..Default::default()
        }),
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
    let addr = "bad-addr".to_string();
    let expected_error = format!("invalid bind address: {}", addr);
    let ingress = IngressSpec {
        bind_admin: Some(BindAdminSpec {
            addr,
            ..Default::default()
        }),
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
    let addr = "127.0.0.1:9000".to_string();
    let expected_error = format!("duplicate bind address: {}", addr);
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr: addr.clone(),
            ..Default::default()
        }),
        bind_admin: Some(BindAdminSpec {
            addr,
            ..Default::default()
        }),
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
    let ingress = IngressSpec {
        bind: Some(BindSpec {
            addr,
            redirect_http_to_https: Some(RedirectSpec {
                port: 8080,
                status: 308,
            }),
            ..Default::default()
        }),
        services: vec![minimal_service()],
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error);
    assert_eq!(report.errors[0].help, expected_help);
}
