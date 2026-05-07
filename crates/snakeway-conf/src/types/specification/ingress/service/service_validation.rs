use crate::types::{HclOrigin, ServiceSpec};
use crate::validation::ValidationReportExt;
use confval::{ValidateSpec, ValidationReport};
use std::collections::HashSet;

impl ValidateSpec<HclOrigin> for ServiceSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        // Validate circuit breaker.
        if let Some(cb) = &self.circuit_breaker
            && cb.enable_auto_recovery
        {
            cb.validate(origin, report);
        }

        // Route validation.
        for route in &self.routes {
            route.validate(origin, report);
        }

        // Upstream validation.
        if self.upstreams.is_empty() {
            report.service_has_no_upstreams(origin);
        }

        let mut seen_sock_values = HashSet::new();

        for upstream in &self.upstreams {
            upstream.validate(&upstream.origin, report);

            // todo move most of this upstream validation into upstream.validate().
            if let (Some(sock), Some(endpoint)) = (&upstream.sock, &upstream.endpoint) {
                report.upstream_cannot_have_both_sock_and_endpoint(
                    sock,
                    &endpoint.host.to_string(),
                    endpoint.port,
                    origin,
                );
                continue;
            }

            if upstream.sock.is_none() && upstream.endpoint.is_none() {
                report.upstream_must_have_a_sock_or_endpoint(origin);
                continue;
            }

            if let Some(endpoint) = &upstream.endpoint {
                endpoint.validate(&upstream.origin, report);

                // Cross-field TLS checks that depend on verify flag.
                if let Some(tls) = &endpoint.tls
                    && tls.verify
                {
                    if tls.sni.parse::<std::net::IpAddr>().is_ok() {
                        report.upstream_tls_sni_must_be_dns(&upstream.origin);
                    }

                    if let Some(ca_file) = &tls.ca_file
                        && let Err(e) = crate::validation::validator::validate_cert_pem(ca_file)
                    {
                        report.upstream_tls_has_invalid_ca_file(ca_file, &e, &upstream.origin);
                    }
                }
            }

            if let Some(sock) = &upstream.sock
                && !seen_sock_values.insert(sock.clone())
            {
                report.duplicate_upstream_sock(sock, &upstream.origin);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        EndpointSpec, EndpointTlsSpec, HclOrigin, HostSpec, ServiceRouteSpec, ServiceSpec,
        UpstreamSpec,
    };
    use confval::{ValidateSpec, ValidationReport};
    use std::net::IpAddr;
    use std::str::FromStr;

    fn minimal_upstream() -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(EndpointSpec {
                host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                port: 3000,
                tls: None,
            }),
            weight: 1,
            ..Default::default()
        }
    }

    fn minimal_service() -> ServiceSpec {
        ServiceSpec {
            upstreams: vec![minimal_upstream()],
            ..Default::default()
        }
    }

    #[test]
    fn valid_minimum_service() {
        // Arrange
        let mut report = ValidationReport::default();
        let service = minimal_service();
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn service_must_have_an_upstream() {
        // Arrange
        let mut report = ValidationReport::default();
        let service = ServiceSpec {
            upstreams: vec![],
            ..Default::default()
        };
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("service has no upstream backends"));
    }

    #[test]
    fn valid_websocket_service() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut service = minimal_service();
        service.routes.push(ServiceRouteSpec {
            hosts: vec!["ws.example.com".to_string()],
            path: "/ws".to_string(),
            enable_websocket: true,
            ws_max_connections: Some(1_000),
            ..Default::default()
        });
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn upstream_cannot_have_both_endpoint_and_sock() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut service = minimal_service();
        service.upstreams[0].endpoint = Some(EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 3000,
            tls: None,
        });
        service.upstreams[0].sock = Some("/tmp/test.sock".to_string());
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        assert_eq!(
            report.errors()[0].message,
            "upstream cannot have both sock /tmp/test.sock and endpoint: 127.0.0.1:3000"
        );
    }

    #[test]
    fn upstream_must_have_either_endpoint_or_sock() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut upstream = minimal_upstream();
        upstream.endpoint = None;
        upstream.sock = None;
        let service = ServiceSpec {
            upstreams: vec![upstream],
            ..Default::default()
        };
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        assert_eq!(
            report.errors()[0].message,
            "invalid upstream - it must have a sock or an endpoint, but neither are defined"
        );
    }

    #[test]
    fn duplicate_upstream_socks_within_service() {
        // Arrange
        let mut report = ValidationReport::default();
        let service = ServiceSpec {
            upstreams: vec![
                UpstreamSpec {
                    sock: Some("/tmp/test.sock".to_string()),
                    weight: 1,
                    ..Default::default()
                },
                UpstreamSpec {
                    sock: Some("/tmp/test.sock".to_string()),
                    weight: 1,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("duplicate upstream sock"));
    }

    #[test]
    fn route_with_no_hosts_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut service = minimal_service();
        service.routes.push(ServiceRouteSpec {
            hosts: vec![],
            path: "/".to_string(),
            ..Default::default()
        });
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("route has no hosts"));
    }

    #[test]
    fn tls_sni_as_ip_rejected_when_verify_true() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut service = minimal_service();
        service.upstreams[0].endpoint = Some(EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 3000,
            tls: Some(EndpointTlsSpec {
                sni: "127.0.0.1".to_string(),
                verify: true,
                ca_file: None,
            }),
        });
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("upstream TLS SNI must be DNS name"));
    }

    #[test]
    fn tls_ca_file_invalid_when_verify_true() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut service = minimal_service();
        service.upstreams[0].endpoint = Some(EndpointSpec {
            host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
            port: 3000,
            tls: Some(EndpointTlsSpec {
                sni: "example.com".to_string(),
                verify: true,
                ca_file: Some("/nonexistent/ca.pem".into()),
            }),
        });
        let origin = HclOrigin::test("service");

        // Act
        service.validate(&origin, &mut report);

        // Assert
        let error = report
            .errors()
            .first()
            .expect("expected at least one error");
        assert!(error.message.contains("upstream TLS has invalid CA file"));
    }
}
