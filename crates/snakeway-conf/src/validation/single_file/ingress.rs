use crate::types::{BindInterfaceSpec, IngressSpec, report_admin_auth_missing};
use confval::prelude::{Located, Report, Span, Validate};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Record `key` as seen at `span`, or report a duplicate against the first
/// occurrence. The message is built lazily, only when a duplicate is found.
fn report_duplicate<K, F>(
    seen: &mut HashMap<K, Span>,
    key: K,
    span: Span,
    report: &mut Report,
    message: F,
) where
    K: Eq + Hash,
    F: FnOnce() -> String,
{
    if let Some(first) = seen.get(&key) {
        report
            .error(message())
            .at(span)
            .related(*first, "first declared here")
            .emit();
    } else {
        seen.insert(key, span);
    }
}

pub(crate) fn validate_ingresses(ingresses: &[Located<IngressSpec>], report: &mut Report) {
    let mut seen_listener_keys: HashMap<String, Span> = HashMap::new();
    let mut seen_redirect_ports: HashMap<i64, Span> = HashMap::new();
    let mut seen_upstream_socks: HashMap<String, Span> = HashMap::new();

    for ingress in ingresses {
        validate_ingress_entity(ingress, report);

        let maybe_bind = ingress.value.bind.as_ref();
        let maybe_bind_admin = ingress.value.bind_admin.as_ref();

        if maybe_bind.is_none() && maybe_bind_admin.is_none() {
            report
                .error("ingress config must have a bind or bind_admin declaration")
                .at(ingress.span)
                .emit();
        }

        if let Some(bind) = maybe_bind {
            validate_listener_uniqueness(
                &bind.value.interface.value,
                bind.value.port.value,
                bind.span,
                report,
                &mut seen_listener_keys,
            );

            if let Some(redirect) = &bind.value.redirect_http_to_https {
                let port = redirect.value.port.value;
                report_duplicate(
                    &mut seen_redirect_ports,
                    port,
                    redirect.value.port.span,
                    report,
                    || format!("duplicate redirect_http_to_https port: {}", port),
                );
            }
        }

        if let Some(bind_admin) = maybe_bind_admin {
            validate_listener_uniqueness(
                &bind_admin.value.interface.value,
                bind_admin.value.port.value,
                bind_admin.span,
                report,
                &mut seen_listener_keys,
            );
        }

        let bind_uses_http2 = maybe_bind.is_some_and(|b| b.value.enable_http2.value);
        let mut seen_route_paths = HashSet::new();
        for service in &ingress.value.services {
            for route in &service.value.routes {
                if !seen_route_paths.insert(route.value.path.value.clone()) {
                    report
                        .error(format!(
                            "duplicate route path within the same listener: {}",
                            route.value.path.value
                        ))
                        .at(route.value.path.span)
                        .help(
                            "Each route path must be unique per listener. Use different path \
                             prefixes or move the route to a separate ingress file.",
                        )
                        .emit();
                }

                if bind_uses_http2 && route.value.enable_websocket.value {
                    report
                        .error(format!(
                            "websocket route cannot be used with HTTP2: {}",
                            route.value.path.value
                        ))
                        .at(route.value.enable_websocket.span)
                        .emit();
                }
            }

            for upstream in &service.value.upstreams {
                if let Some(sock) = &upstream.value.sock {
                    report_duplicate(
                        &mut seen_upstream_socks,
                        sock.value.clone(),
                        sock.span,
                        report,
                        || format!("duplicate upstream sock: {}", sock.value),
                    );
                }
            }
        }
    }
}

fn validate_ingress_entity(ingress: &Located<IngressSpec>, report: &mut Report) {
    // Field-local validation for every child entity, dispatched compositionally.
    ingress.value.validate(report);

    // Presence checks that need an enclosing span the child cannot reach from
    // `&self` stay here, where the `Located` wrappers are in hand.
    if let Some(bind_admin) = &ingress.value.bind_admin
        && bind_admin.value.auth.is_none()
    {
        report_admin_auth_missing(bind_admin.span, report);
    }

    for service in &ingress.value.services {
        if service.value.upstreams.is_empty() {
            report
                .error("service has no upstream backends")
                .at(service.span)
                .emit();
        }
    }
}

fn validate_listener_uniqueness(
    interface: &str,
    port: i64,
    span: Span,
    report: &mut Report,
    seen_listener_keys: &mut HashMap<String, Span>,
) {
    let maybe_interface: Result<BindInterfaceSpec, _> = interface.try_into();
    if let Ok(interface) = maybe_interface {
        let key = interface.socket_address_literal(port as u16);
        report_duplicate(seen_listener_keys, key.clone(), span, report, || {
            format!("duplicate bind address: {}", key)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::validate_ingresses;
    use crate::types::*;
    use confval::prelude::{Located, Report};

    fn minimal_service() -> Located<ServiceSpec> {
        Located::detached(ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                path: Located::detached("/".to_string()),
                hosts: vec![Located::detached("example.com".to_string())],
                ..Default::default()
            })],
            upstreams: vec![Located::detached(UpstreamSpec {
                endpoint: Some(Located::detached(EndpointSpec {
                    host: Located::detached("127.0.0.1".to_string()),
                    port: Located::detached(8080),
                    tls: None,
                })),
                sock: None,
                weight: Located::detached(1),
            })],
            load_balancing_strategy: Located::detached("failover".to_string()),
            ..Default::default()
        })
    }

    fn minimal_bind() -> BindSpec {
        BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        }
    }

    fn minimal_ingress() -> Located<IngressSpec> {
        Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![minimal_service()],
            ..Default::default()
        })
    }

    fn sock_upstream(sock: &str) -> Located<UpstreamSpec> {
        Located::detached(UpstreamSpec {
            endpoint: None,
            sock: Some(Located::detached(sock.to_string())),
            weight: Located::detached(1),
        })
    }

    // The "no upstream backends" check needs the enclosing service span, so it
    // lives here rather than in ServiceSpec::validate.
    #[test]
    fn service_must_have_an_upstream() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![Located::detached(ServiceSpec {
                load_balancing_strategy: Located::detached("failover".to_string()),
                upstreams: vec![],
                ..Default::default()
            })],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "service has no upstream backends")
        );
    }

    // The auth-absent check needs the enclosing bind_admin span, so it lives
    // here rather than in BindAdminSpec::validate.
    #[test]
    fn missing_auth_block_produces_error() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(9000),
                ..Default::default()
            })),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "bind_admin.auth is required")
        );
    }

    #[test]
    fn duplicate_bind_addr() {
        // Arrange
        let mut report = Report::new();
        let ingress1 = minimal_ingress();
        let ingress2 = minimal_ingress();

        // Act
        validate_ingresses(&[ingress1, ingress2], &mut report);

        // Assert
        assert_eq!(
            report.issues()[0].message,
            "duplicate bind address: 127.0.0.1:8080"
        );
    }

    #[test]
    fn duplicate_admin_and_public_bind() {
        // Arrange
        let mut report = Report::new();
        let port = 9000;
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(BindSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(port),
                ..Default::default()
            })),
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(port),
                ..Default::default()
            })),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == format!("duplicate bind address: 127.0.0.1:{}", port))
        );
    }

    #[test]
    fn sock_file_not_reused_across_services() {
        // Arrange
        let mut report = Report::new();
        let expected_error = "duplicate upstream sock: /tmp/shared.sock";
        let make_service = || {
            Located::detached(ServiceSpec {
                routes: vec![Located::detached(ServiceRouteSpec {
                    path: Located::detached("/".to_string()),
                    hosts: vec![Located::detached("example.com".to_string())],
                    ..Default::default()
                })],
                upstreams: vec![sock_upstream("/tmp/shared.sock")],
                load_balancing_strategy: Located::detached("failover".to_string()),
                ..Default::default()
            })
        };
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![make_service(), make_service()],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(report.issues().iter().any(|e| e.message == expected_error));
    }

    #[test]
    fn ingress_missing_bind_produces_error() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind: None,
            bind_admin: None,
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "ingress config must have a bind or bind_admin declaration")
        );
    }

    #[test]
    fn http2_with_websocket_route_produces_error() {
        // Arrange
        let mut report = Report::new();
        let mut bind = minimal_bind();
        bind.enable_http2 = Located::detached(true);
        bind.tls = Some(Located::detached(TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
        }));
        let service = Located::detached(ServiceSpec {
            routes: vec![Located::detached(ServiceRouteSpec {
                path: Located::detached("/ws".to_string()),
                hosts: vec![Located::detached("ws.example.com".to_string())],
                enable_websocket: Located::detached(true),
                ws_max_connections: None,
            })],
            upstreams: vec![sock_upstream("/tmp/ws.sock")],
            load_balancing_strategy: Located::detached("failover".to_string()),
            ..Default::default()
        });
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![service],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "websocket route cannot be used with HTTP2: /ws")
        );
    }

    #[test]
    fn duplicate_redirect_ports_across_ingresses() {
        // Arrange
        let mut report = Report::new();
        let make_ingress = |bind_port: i64| {
            let mut bind = minimal_bind();
            bind.port = Located::detached(bind_port);
            bind.tls = Some(Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached("example.com".to_string())],
                challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
            }));
            bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
                port: Located::detached(9090),
                status: Located::detached(308),
            }));
            Located::detached(IngressSpec {
                bind: Some(Located::detached(bind)),
                services: vec![minimal_service()],
                ..Default::default()
            })
        };

        // Act
        validate_ingresses(&[make_ingress(8080), make_ingress(8081)], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "duplicate redirect_http_to_https port: 9090")
        );
    }

    #[test]
    fn validate_multiple_services_at_once() {
        // Arrange
        let mut report = Report::new();
        let empty_service = Located::detached(ServiceSpec {
            routes: vec![],
            upstreams: vec![],
            load_balancing_strategy: Located::detached("failover".to_string()),
            ..Default::default()
        });
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![minimal_service(), empty_service],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("service has no upstream backends"))
        );
    }

    #[test]
    fn duplicate_route_path_within_same_ingress() {
        // Arrange
        let mut report = Report::new();
        let route = || {
            Located::detached(ServiceRouteSpec {
                path: Located::detached("/api".to_string()),
                hosts: vec![Located::detached("example.com".to_string())],
                ..Default::default()
            })
        };
        let service = Located::detached(ServiceSpec {
            routes: vec![route(), route()],
            upstreams: vec![sock_upstream("/tmp/api.sock")],
            load_balancing_strategy: Located::detached("failover".to_string()),
            ..Default::default()
        });
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![service],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate route path"))
        );
    }

    #[test]
    fn same_route_path_on_different_ingresses_is_allowed() {
        // Arrange
        let mut report = Report::new();
        let make_ingress = |port: i64| {
            let mut bind = minimal_bind();
            bind.port = Located::detached(port);
            Located::detached(IngressSpec {
                bind: Some(Located::detached(bind)),
                services: vec![minimal_service()],
                ..Default::default()
            })
        };

        // Act
        validate_ingresses(&[make_ingress(8080), make_ingress(8081)], &mut report);

        // Assert
        assert!(
            !report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate route path")),
            "issues: {:?}",
            report.issues()
        );
    }
}
