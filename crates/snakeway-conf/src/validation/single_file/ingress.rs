use crate::types::{BindInterfaceInput, BindInterfaceSpec, IngressSpec, OriginDeprecated};
use crate::validation::{ValidateSpec, ValidationReportDeprecated};
use std::collections::HashSet;

pub(crate) fn validate_ingresses(
    ingresses: &[IngressSpec],
    report: &mut ValidationReportDeprecated,
) {
    let mut seen_listener_keys = HashSet::new();
    let mut seen_redirect_ports = HashSet::new();
    let mut seen_upstream_socks = HashSet::new();

    for ingress in ingresses {
        // Ingress validation.
        ingress.validate(&ingress.origin, report);

        // Cross-ingress validation checks depend on the ingress's bind and/or bind_admin.
        let maybe_bind = ingress.bind.as_ref();
        let maybe_bind_admin = ingress.bind_admin.as_ref();

        //---------------------------------------------------------------------
        // Bind/Admin bind presence check.
        // There must be at least one bind or admin bind.
        //---------------------------------------------------------------------
        if maybe_bind.is_none() && maybe_bind_admin.is_none() {
            report.missing_bind(&ingress.origin);
        }

        //---------------------------------------------------------------------
        // Bind uniqueness checks.
        //---------------------------------------------------------------------
        if let Some(bind) = maybe_bind {
            // Validate listener uniqueness.
            validate_listener_uniqueness(
                &bind.interface,
                bind.port,
                &bind.origin,
                report,
                &mut seen_listener_keys,
            );

            // Validate redirects' port uniqueness.
            if let Some(redirect) = &bind.redirect_http_to_https
                && !seen_redirect_ports.insert(redirect.port)
            {
                report.duplicate_redirect_http_to_https_port(redirect.port, &bind.origin);
            }
        }

        //---------------------------------------------------------------------
        // Admin bind uniqueness checks.
        //---------------------------------------------------------------------
        if let Some(bind_admin) = maybe_bind_admin {
            validate_listener_uniqueness(
                &bind_admin.interface,
                bind_admin.port,
                &bind_admin.origin,
                report,
                &mut seen_listener_keys,
            );
        }

        //---------------------------------------------------------------------
        // Duplicate route paths within the same ingress.
        // The router uses path as the primary lookup key within a listener,
        // so two services cannot share the same route path prefix.
        //---------------------------------------------------------------------
        let mut seen_route_paths = HashSet::new();
        for service in &ingress.services {
            for route in &service.routes {
                if !seen_route_paths.insert(&route.path) {
                    report.duplicate_route_path(&route.path, &route.origin);
                }
            }
        }

        //---------------------------------------------------------------------
        // Bind/Route http2/websocket agreement.
        // If bind has http2 enabled, websocket routes cannot be used.
        //---------------------------------------------------------------------
        let bind_uses_http2 = ingress.bind.as_ref().is_some_and(|b| b.enable_http2);
        for service in &ingress.services {
            for route in &service.routes {
                if bind_uses_http2 && route.enable_websocket {
                    report.websocket_route_cannot_be_used_with_http2(&route.path, &route.origin);
                }
            }
        }

        //---------------------------------------------------------------------
        // Cross-ingress upstream sock uniqueness
        //---------------------------------------------------------------------
        for service in &ingress.services {
            for upstream in &service.upstreams {
                if let Some(sock) = &upstream.sock
                    && !seen_upstream_socks.insert(sock.clone())
                {
                    report.duplicate_upstream_sock(sock, &service.origin);
                }
            }
        }
    }
}

/// Verify that a socket address (ip:port) is not used more than once.
fn validate_listener_uniqueness(
    bind_interface_input: &BindInterfaceInput,
    port: u16,
    origin: &OriginDeprecated,
    report: &mut ValidationReportDeprecated,
    seen_listener_keys: &mut HashSet<String>,
) {
    let maybe_interface: Result<BindInterfaceSpec, _> = bind_interface_input.clone().try_into();
    if let Ok(interface) = maybe_interface {
        let key = interface.socket_address_literal(port);
        if !seen_listener_keys.insert(key.clone()) {
            report.duplicate_bind_addr(&key, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_ingresses;
    use crate::types::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    use crate::validation::ValidationReportDeprecated;

    fn minimal_service() -> ServiceSpec {
        ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/".to_string(),
                hosts: vec!["example.com".to_string()],
                ..Default::default()
            }],
            upstreams: vec![UpstreamSpec {
                endpoint: Some(EndpointSpec {
                    host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                    port: 8080,
                    tls: None,
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

    fn minimal_ingress() -> IngressSpec {
        IngressSpec {
            bind: Some(minimal_bind()),
            services: vec![minimal_service()],
            ..Default::default()
        }
    }

    #[test]
    fn duplicate_bind_addr() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let ingress1 = minimal_ingress();
        let ingress2 = minimal_ingress();

        // Act
        validate_ingresses(&[ingress1, ingress2], &mut report);

        // Assert
        assert_eq!(
            report.errors[0].message,
            "duplicate bind address: 127.0.0.1:8080"
        );
    }

    #[test]
    fn duplicate_admin_and_public_bind() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let port = 9000;
        let interface = BindInterfaceInput::Keyword("loopback".to_string());
        let ingress = IngressSpec {
            bind: Some(BindSpec {
                interface: interface.clone(),
                port,
                ..Default::default()
            }),
            bind_admin: Some(BindAdminSpec {
                interface: interface.clone(),
                port,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == format!("duplicate bind address: 127.0.0.1:{}", port))
        );
    }

    #[test]
    fn sock_file_not_reused_across_services() {
        // Arrange
        let sock = "/tmp/test.sock".to_string();
        let expected_error = format!("duplicate upstream sock: {}", sock);
        let mut report = ValidationReportDeprecated::default();
        let ingress = IngressSpec {
            bind: Some(minimal_bind()),
            services: vec![
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
            ],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(report.errors.iter().any(|e| e.message == expected_error));
    }

    #[test]
    fn ingress_missing_bind_produces_error() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let ingress = IngressSpec {
            bind: None,
            bind_admin: None,
            services: vec![minimal_service()],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "ingress config must have a bind or bind_admin declaration")
        );
    }

    #[test]
    fn http2_with_websocket_route_produces_error() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, cert_pem).unwrap();
        std::fs::write(&key_path, key_pem).unwrap();

        let ingress = IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 8443,
                tls: Some(TlsTerminationSpec::Manual {
                    cert: cert_path,
                    key: key_path,
                }),
                enable_http2: true,
                ..Default::default()
            }),
            services: vec![ServiceSpec {
                routes: vec![ServiceRouteSpec {
                    path: "/ws".to_string(),
                    hosts: vec!["example.com".to_string()],
                    enable_websocket: true,
                    ..Default::default()
                }],
                upstreams: vec![UpstreamSpec {
                    endpoint: Some(EndpointSpec {
                        host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                        port: 9090,
                        tls: None,
                    }),
                    weight: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "websocket route cannot be used with HTTP2: /ws")
        );
    }

    #[test]
    fn duplicate_redirect_ports_across_ingresses() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, &cert_pem).unwrap();
        std::fs::write(&key_path, &key_pem).unwrap();

        let make_ingress = |port: u16| IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port,
                tls: Some(TlsTerminationSpec::Manual {
                    cert: cert_path.clone(),
                    key: key_path.clone(),
                }),
                redirect_http_to_https: Some(RedirectSpec {
                    port: 9090,
                    status: 308,
                }),
                ..Default::default()
            }),
            services: vec![minimal_service()],
            ..Default::default()
        };

        let ingress1 = make_ingress(8443);
        let ingress2 = make_ingress(8444);

        // Act
        validate_ingresses(&[ingress1, ingress2], &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "duplicate redirect_http_to_https port: 9090")
        );
    }

    #[test]
    fn validate_multiple_services_at_once() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let ingress = IngressSpec {
            bind: Some(minimal_bind()),
            services: vec![
                minimal_service(),
                ServiceSpec {
                    origin: OriginDeprecated {
                        section: "service_2".to_string(),
                        ..Default::default()
                    },
                    upstreams: vec![], // Invalid: no upstreams
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("service has no upstream backends"))
        );
    }

    #[test]
    fn duplicate_route_path_within_same_ingress() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let ingress = IngressSpec {
            bind: Some(minimal_bind()),
            services: vec![
                ServiceSpec {
                    routes: vec![ServiceRouteSpec {
                        path: "/api".to_string(),
                        hosts: vec!["a.test".to_string()],
                        ..Default::default()
                    }],
                    upstreams: vec![UpstreamSpec {
                        endpoint: Some(EndpointSpec {
                            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                            port: 8080,
                            tls: None,
                        }),
                        weight: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ServiceSpec {
                    routes: vec![ServiceRouteSpec {
                        path: "/api".to_string(),
                        hosts: vec!["b.test".to_string()],
                        ..Default::default()
                    }],
                    upstreams: vec![UpstreamSpec {
                        endpoint: Some(EndpointSpec {
                            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                            port: 9090,
                            tls: None,
                        }),
                        weight: 1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(report.errors.iter().any(|e| {
            e.message
                .contains("duplicate route path within the same listener: /api")
        }));
    }

    #[test]
    fn same_route_path_on_different_ingresses_is_allowed() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let make_ingress = |port: u16| IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port,
                ..Default::default()
            }),
            services: vec![ServiceSpec {
                routes: vec![ServiceRouteSpec {
                    path: "/api".to_string(),
                    hosts: vec!["example.com".to_string()],
                    ..Default::default()
                }],
                upstreams: vec![UpstreamSpec {
                    endpoint: Some(EndpointSpec {
                        host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                        port: 8080,
                        tls: None,
                    }),
                    weight: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Act
        validate_ingresses(&[make_ingress(8080), make_ingress(9090)], &mut report);

        // Assert
        assert!(
            !report
                .errors
                .iter()
                .any(|e| e.message.contains("duplicate route path")),
            "same path on different listeners should be allowed"
        );
    }
}
