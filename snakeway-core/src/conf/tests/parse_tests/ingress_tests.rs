use crate::conf::parse::parse_ingress;
use crate::conf::validation::ConfigError;
use std::fs;
use tempfile::tempdir;

#[test]
fn parse_ingress_bind_file() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("api.hcl");

    fs::write(
        &path,
        r#"
bind = {
  addr = "127.0.0.1:8443"
  enable_http2 = true
  tls = {
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#,
    )
    .unwrap();

    // Act
    let ingress = parse_ingress(&path).unwrap();

    // Assert
    let bind = ingress.bind.unwrap();
    assert_eq!(bind.origin.section, "bind");
}

#[test]
fn parse_ingress_admin_bind_file() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("admin.hcl");

    fs::write(
        &path,
        r#"
bind_admin = {
  addr = "127.0.0.1:8440"
  tls = {
    cert = "cert.pem"
    key  = "key.pem"
  }
}
"#,
    )
    .unwrap();

    // Act
    let ingress = parse_ingress(&path).unwrap();

    // Assert
    let bind_admin = ingress.bind_admin.unwrap();
    assert_eq!(bind_admin.origin.section, "bind_admin");
}

#[test]
fn parse_ingress_services_and_routes_have_origin() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("api.hcl");

    fs::write(
        &path,
        r#"
services = [
  {
    routes = [
      { path = "/api" },
      { path = "/ws" }
    ]

    upstreams = [
      { addr = "127.0.0.1:3000" }
    ]
  }
]
"#,
    )
    .unwrap();

    // Act
    let ingress = parse_ingress(&path).unwrap();

    // Assert
    let svc = &ingress.services[0];
    assert_eq!(svc.origin.section, "service");
    assert_eq!(svc.origin.index, Some(0));

    assert_eq!(svc.routes[0].origin.section, "route");
    assert_eq!(svc.routes[0].origin.index, Some(0));

    assert_eq!(svc.routes[1].origin.index, Some(1));
    assert_eq!(svc.upstreams[0].origin.section, "backend");
}

#[test]
fn parse_ingress_invalid_hcl_returns_error() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.hcl");

    fs::write(&path, "services = [").unwrap();

    // Act
    let err = parse_ingress(&path).unwrap_err();

    // Assert
    assert!(matches!(err, ConfigError::Parse { .. }));
}
