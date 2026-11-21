use crate::types::{
    CircuitBreakerSpec, HealthCheckSpec, HostSpec, LoadBalancingStrategy, ServiceRouteSpec,
    UpstreamSpec,
};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct ServiceSpec {
    /// The service's unique name across all ingress files.
    #[confval(label, derive_default = String::new())]
    pub name: Located<String>,
    #[confval(default = "failover".to_string(), keywords = LoadBalancingStrategy)]
    pub load_balancing_strategy: Located<String>,
    #[confval(nested)]
    pub routes: Vec<Located<ServiceRouteSpec>>,
    #[confval(nested)]
    pub upstreams: Vec<Located<UpstreamSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub health_check: Option<Located<HealthCheckSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub circuit_breaker: Option<Located<CircuitBreakerSpec>>,
}

impl Validate for ServiceSpec {
    fn validate(&self, report: &mut Report) {
        // A route with no hosts is reported at the route's enclosing span,
        // which the route cannot reach from `&self`.
        for route in &self.routes {
            if route.value.hosts.is_empty() {
                report.error("route has no hosts").at(route.span).emit();
            }
        }

        let mut seen_endpoints = HashMap::new();

        for upstream in &self.upstreams {
            if let (Some(sock), Some(endpoint)) = (&upstream.value.sock, &upstream.value.endpoint) {
                report
                    .error(format!(
                        "upstream cannot have both sock: {} and endpoint: {}:{}",
                        sock.value.path.value, endpoint.value.host.value, endpoint.value.port.value
                    ))
                    .at(upstream.span)
                    .emit();
                continue;
            }

            if upstream.value.sock.is_none() && upstream.value.endpoint.is_none() {
                report
                    .error(
                        "invalid upstream - it must have a sock or an endpoint, \
                         but neither are defined",
                    )
                    .at(upstream.span)
                    .help("Only one can be set.")
                    .emit();
                continue;
            }

            if let Some(endpoint) = &upstream.value.endpoint {
                let key = (
                    HostSpec::parse(&endpoint.value.host.value),
                    endpoint.value.port.value,
                );
                if let Some(first) = seen_endpoints.get(&key) {
                    report
                        .error(format!(
                            "duplicate upstream endpoint: {}:{}",
                            endpoint.value.host.value, endpoint.value.port.value
                        ))
                        .at(endpoint.span)
                        .related(*first, "first declared here")
                        .emit();
                } else {
                    seen_endpoints.insert(key, endpoint.span);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BindSpec, EndpointSpec, EndpointTlsSpec, IngressSpec, SockSpec};

    fn minimal_upstream() -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(Located::detached(EndpointSpec {
                host: Located::detached("127.0.0.1".to_string()),
                port: Located::detached(3000),
                tls: None,
            })),
            sock: None,
            weight: Located::detached(1),
        }
    }

    fn minimal_service() -> ServiceSpec {
        ServiceSpec {
            name: Located::detached("api".to_string()),
            upstreams: vec![Located::detached(minimal_upstream())],
            ..Default::default()
        }
    }

    fn validate(spec: &ServiceSpec) -> Report {
        let mut report = Report::new();
        spec.validate_all(&mut report);
        report
    }

    #[test]
    fn valid_minimum_service() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("failover".to_string());

        // Act
        let report = validate(&service);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn unknown_load_balancing_strategy_rejected() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("psychic".to_string());

        // Act
        let report = validate(&service);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown load_balancing_strategy: psychic")
        );
    }

    #[test]
    fn valid_websocket_service() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("failover".to_string());
        service.routes.push(Located::detached(ServiceRouteSpec {
            hosts: vec![Located::detached("ws.example.com".to_string())],
            path: Located::detached("/ws".to_string()),
            enable_websocket: Located::detached(true),
            ws_max_connections: Some(Located::detached(1_000)),
        }));

        // Act
        let report = validate(&service);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn upstream_cannot_have_both_endpoint_and_sock() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("failover".to_string());
        service.upstreams[0].value.sock = Some(Located::detached(SockSpec {
            path: Located::detached("/tmp/test.sock".to_string()),
            tls: None,
        }));

        // Act
        let report = validate(&service);

        // Assert
        assert_eq!(
            report.issues()[0].message,
            "upstream cannot have both sock: /tmp/test.sock and endpoint: 127.0.0.1:3000"
        );
    }

    #[test]
    fn upstream_must_have_either_endpoint_or_sock() {
        // Arrange
        let mut upstream = minimal_upstream();
        upstream.endpoint = None;
        upstream.sock = None;
        let service = ServiceSpec {
            name: Located::detached("api".to_string()),
            load_balancing_strategy: Located::detached("failover".to_string()),
            upstreams: vec![Located::detached(upstream)],
            ..Default::default()
        };

        // Act
        let report = validate(&service);

        // Assert
        assert_eq!(
            report.issues()[0].message,
            "invalid upstream - it must have a sock or an endpoint, but neither are defined"
        );
    }

    #[test]
    fn duplicate_upstream_socks_within_service() {
        // Arrange
        let sock_upstream = || {
            Located::detached(UpstreamSpec {
                endpoint: None,
                sock: Some(Located::detached(SockSpec {
                    path: Located::detached("/tmp/test.sock".to_string()),
                    tls: None,
                })),
                weight: Located::detached(1),
            })
        };
        let service = ServiceSpec {
            name: Located::detached("api".to_string()),
            load_balancing_strategy: Located::detached("failover".to_string()),
            upstreams: vec![sock_upstream(), sock_upstream()],
            ..Default::default()
        };
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(BindSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(8080),
                ..Default::default()
            })),
            services: vec![Located::detached(service)],
            ..Default::default()
        });
        let mut report = Report::new();

        // Act
        crate::validation::validate_ingresses(&[ingress], &mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("duplicate upstream sock"));
    }

    fn endpoint_upstream(host: &str, port: i64) -> Located<UpstreamSpec> {
        Located::detached(UpstreamSpec {
            endpoint: Some(Located::detached(EndpointSpec {
                host: Located::detached(host.to_string()),
                port: Located::detached(port),
                tls: None,
            })),
            sock: None,
            weight: Located::detached(1),
        })
    }

    #[test]
    fn duplicate_upstream_endpoints_within_service() {
        // Arrange
        let service = ServiceSpec {
            name: Located::detached("api".to_string()),
            upstreams: vec![
                endpoint_upstream("10.0.0.1", 3000),
                endpoint_upstream("10.0.0.2", 3000),
                endpoint_upstream("10.0.0.1", 3000),
            ],
            ..Default::default()
        };

        // Act
        let report = validate(&service);

        // Assert
        let messages: Vec<&str> = report.issues().iter().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, ["duplicate upstream endpoint: 10.0.0.1:3000"]);
    }

    #[test]
    fn upstream_endpoint_hosts_that_differ_only_in_case_are_duplicates() {
        // Arrange
        let service = ServiceSpec {
            name: Located::detached("api".to_string()),
            upstreams: vec![
                endpoint_upstream("app.example.com", 3000),
                endpoint_upstream("App.Example.com", 3000),
            ],
            ..Default::default()
        };

        // Act
        let report = validate(&service);

        // Assert
        let messages: Vec<&str> = report.issues().iter().map(|e| e.message.as_str()).collect();
        assert_eq!(
            messages,
            ["duplicate upstream endpoint: App.Example.com:3000"]
        );
    }

    #[test]
    fn upstream_endpoints_on_the_same_host_with_different_ports_are_accepted() {
        // Arrange
        let service = ServiceSpec {
            name: Located::detached("api".to_string()),
            upstreams: vec![
                endpoint_upstream("10.0.0.1", 3000),
                endpoint_upstream("10.0.0.1", 3001),
            ],
            ..Default::default()
        };

        // Act
        let report = validate(&service);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    fn service_with_sock_upstream(sock: SockSpec) -> ServiceSpec {
        ServiceSpec {
            name: Located::detached("api".to_string()),
            load_balancing_strategy: Located::detached("failover".to_string()),
            upstreams: vec![Located::detached(UpstreamSpec {
                endpoint: None,
                sock: Some(Located::detached(sock)),
                weight: Located::detached(1),
            })],
            ..Default::default()
        }
    }

    fn sock_tls_with_sni(sni: &str) -> Option<Located<EndpointTlsSpec>> {
        Some(Located::detached(EndpointTlsSpec {
            sni: Located::detached(sni.to_string()),
            verify: Located::detached(true),
            ca_file: None,
        }))
    }

    #[test]
    fn empty_sock_path_is_rejected() {
        // Arrange
        let service = service_with_sock_upstream(SockSpec {
            path: Located::detached(String::new()),
            tls: None,
        });

        // Act
        let report = validate(&service);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "path must not be empty"),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn empty_sni_in_sock_tls_is_rejected() {
        // Arrange
        let service = service_with_sock_upstream(SockSpec {
            path: Located::detached("/run/app.sock".to_string()),
            tls: sock_tls_with_sni(""),
        });

        // Act
        let report = validate(&service);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "sni must not be empty"),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn ip_sni_in_sock_tls_is_rejected_when_verify_is_true() {
        // Arrange
        let service = service_with_sock_upstream(SockSpec {
            path: Located::detached("/run/app.sock".to_string()),
            tls: sock_tls_with_sni("10.0.0.1"),
        });

        // Act
        let report = validate(&service);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "upstream TLS SNI must be DNS name"),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn route_with_no_hosts_produces_error() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("failover".to_string());
        service.routes.push(Located::detached(ServiceRouteSpec {
            hosts: vec![],
            path: Located::detached("/".to_string()),
            ..Default::default()
        }));

        // Act
        let report = validate(&service);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("route has no hosts"));
    }

    #[test]
    fn tls_sni_as_ip_rejected_when_verify_true() {
        // Arrange
        let mut service = minimal_service();
        service.load_balancing_strategy = Located::detached("failover".to_string());
        service.upstreams[0].value.endpoint = Some(Located::detached(EndpointSpec {
            host: Located::detached("127.0.0.1".to_string()),
            port: Located::detached(3000),
            tls: Some(Located::detached(EndpointTlsSpec {
                sni: Located::detached("127.0.0.1".to_string()),
                verify: Located::detached(true),
                ca_file: None,
            })),
        }));

        // Act
        let report = validate(&service);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "upstream TLS SNI must be DNS name")
        );
    }
}
