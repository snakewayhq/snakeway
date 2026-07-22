use super::service_route_spec::validate_service_route;
use super::upstream_spec::{validate_endpoint, validate_endpoint_tls_verify, validate_upstream};
use crate::types::{CircuitBreakerSpec, HealthCheckSpec, ServiceRouteSpec, UpstreamSpec};
use confval::prelude::{KeywordSet, Located, Report, Validate};
use serde::Serialize;
use std::collections::HashSet;

pub const LOAD_BALANCING_STRATEGIES: [&str; 5] = [
    "failover",
    "round_robin",
    "request_pressure",
    "sticky_hash",
    "random",
];

#[derive(Debug, Serialize, confval::Spec)]
pub struct ServiceSpec {
    #[confval(default = "failover".to_string())]
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

impl Default for ServiceSpec {
    fn default() -> Self {
        Self {
            load_balancing_strategy: Located::detached("failover".to_string()),
            routes: Vec::new(),
            upstreams: Vec::new(),
            health_check: None,
            circuit_breaker: None,
        }
    }
}

impl Validate for ServiceSpec {
    fn validate(&self, report: &mut Report) {
        KeywordSet::new(&LOAD_BALANCING_STRATEGIES).check_located(
            &self.load_balancing_strategy,
            "load_balancing_strategy",
            report,
        );

        if let Some(cb) = &self.circuit_breaker
            && cb.value.enable_auto_recovery.value
        {
            cb.validate(report);
        }

        for route in &self.routes {
            validate_service_route(&route.value, route.span, report);
        }

        let mut seen_sock_values = HashSet::new();

        for upstream in &self.upstreams {
            validate_upstream(&upstream.value, report);

            if let (Some(sock), Some(endpoint)) = (&upstream.value.sock, &upstream.value.endpoint) {
                report
                    .error(format!(
                        "upstream cannot have both sock {} and endpoint: {}:{}",
                        sock.value, endpoint.value.host.value, endpoint.value.port.value
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
                validate_endpoint(&endpoint.value, report);

                if let Some(tls) = &endpoint.value.tls {
                    validate_endpoint_tls_verify(&tls.value, report);
                }
            }

            if let Some(sock) = &upstream.value.sock
                && !seen_sock_values.insert(sock.value.clone())
            {
                report
                    .error(format!("duplicate upstream sock: {}", sock.value))
                    .at(sock.span)
                    .emit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EndpointSpec, EndpointTlsSpec};

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
            upstreams: vec![Located::detached(minimal_upstream())],
            ..Default::default()
        }
    }

    fn validate(spec: &ServiceSpec) -> Report {
        let mut report = Report::new();
        spec.validate(&mut report);
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
        service.upstreams[0].value.sock = Some(Located::detached("/tmp/test.sock".to_string()));

        // Act
        let report = validate(&service);

        // Assert
        assert_eq!(
            report.issues()[0].message,
            "upstream cannot have both sock /tmp/test.sock and endpoint: 127.0.0.1:3000"
        );
    }

    #[test]
    fn upstream_must_have_either_endpoint_or_sock() {
        // Arrange
        let mut upstream = minimal_upstream();
        upstream.endpoint = None;
        upstream.sock = None;
        let service = ServiceSpec {
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
                sock: Some(Located::detached("/tmp/test.sock".to_string())),
                weight: Located::detached(1),
            })
        };
        let service = ServiceSpec {
            load_balancing_strategy: Located::detached("failover".to_string()),
            upstreams: vec![sock_upstream(), sock_upstream()],
            ..Default::default()
        };

        // Act
        let report = validate(&service);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("duplicate upstream sock"));
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
