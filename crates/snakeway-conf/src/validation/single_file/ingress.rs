use crate::types::{BindInterfaceSpec, IngressSpec, report_admin_auth_missing};
use confval::prelude::{Located, Report, Span, Validate};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// The socket address literal for a declared interface and port, or `None`
/// when either is not representable. The literal is the shared key that keeps
/// bind, bind_admin, and redirect listeners unique against each other.
fn listener_key(interface: &str, port: i64) -> Option<String> {
    let interface: Result<BindInterfaceSpec, _> = interface.try_into();
    let port = u16::try_from(port).ok()?;
    interface.ok()?.socket_address_literal(port).into()
}

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
    let mut seen_upstream_socks: HashMap<String, Span> = HashMap::new();
    let mut seen_service_names: HashMap<String, Span> = HashMap::new();

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

        // An ingress with only bind_admin has no listener to attach its
        // services and static files to, so lowering drops them. This is a
        // warning rather than an error, so an existing config never starts
        // failing.
        if maybe_bind.is_none() && maybe_bind_admin.is_some() {
            let service_count = ingress.value.services.len();
            let static_file_count = ingress.value.static_files.len();
            if service_count > 0 || static_file_count > 0 {
                report
                    .warning(format!(
                        "this ingress has no bind, so its {} service(s) and {} static file \
                         block(s) are ignored",
                        service_count, static_file_count
                    ))
                    .at(ingress.span)
                    .emit();
            }
        }

        if let Some(bind) = maybe_bind {
            validate_listener_uniqueness(
                &bind.value.interface.value,
                bind.value.port.value,
                bind.span,
                report,
                &mut seen_listener_keys,
            );

            // The redirect listener binds its own socket, so it must be unique
            // against every other listener socket, including the bind it hangs
            // off.
            if let Some(redirect) = &bind.value.redirect_http_to_https
                && let Some(key) =
                    listener_key(&bind.value.interface.value, redirect.value.port.value)
            {
                report_duplicate(
                    &mut seen_listener_keys,
                    key.clone(),
                    redirect.value.port.span,
                    report,
                    || format!("duplicate redirect listener address: {key}"),
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
            // The lowered services keep one global map keyed by name, so the
            // label must be unique across every ingress file, not just its own.
            report_duplicate(
                &mut seen_service_names,
                service.value.name.value.clone(),
                service.value.name.span,
                report,
                || format!("duplicate service name: {}", service.value.name.value),
            );

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
                        sock.value.path.value.clone(),
                        sock.value.path.span,
                        report,
                        || format!("duplicate upstream sock: {}", sock.value.path.value),
                    );
                }
            }
        }

        for static_files in &ingress.value.static_files {
            for route in &static_files.value.routes {
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
            }
        }
    }
}

fn validate_ingress_entity(ingress: &Located<IngressSpec>, report: &mut Report) {
    // Field-local validation for every child entity, dispatched compositionally.
    ingress.validate_all(report);

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
    if let (Ok(interface), Ok(port)) = (maybe_interface, u16::try_from(port)) {
        let key = interface.socket_address_literal(port);
        report_duplicate(seen_listener_keys, key.clone(), span, report, || {
            format!("duplicate bind address: {key}")
        });
    }
}

#[cfg(test)]
mod tests {
    use super::validate_ingresses;
    use crate::types::*;
    use confval::prelude::{Located, Report};
    use tempfile::NamedTempFile;

    fn minimal_service() -> Located<ServiceSpec> {
        Located::detached(ServiceSpec {
            name: Located::detached("api".to_string()),
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
            sock: Some(Located::detached(SockSpec {
                path: Located::detached(sock.to_string()),
                tls: None,
            })),
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
                name: Located::detached("api".to_string()),
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
    fn duplicate_service_name_across_ingresses_is_rejected() {
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
        validate_ingresses(&[make_ingress(8080), make_ingress(9090)], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "duplicate service name: api"),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn sock_file_not_reused_across_services() {
        // Arrange
        let mut report = Report::new();
        let expected_error = "duplicate upstream sock: /tmp/shared.sock";
        let make_service = |name: &str| {
            Located::detached(ServiceSpec {
                name: Located::detached(name.to_string()),
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
            services: vec![make_service("svc-a"), make_service("svc-b")],
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
            challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
        }));
        let service = Located::detached(ServiceSpec {
            name: Located::detached("ws".to_string()),
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
                challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
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
                .any(|e| e.message.contains("duplicate redirect listener address"))
        );
    }

    #[test]
    fn validate_multiple_services_at_once() {
        // Arrange
        let mut report = Report::new();
        let empty_service = Located::detached(ServiceSpec {
            name: Located::detached("empty".to_string()),
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
            name: Located::detached("api".to_string()),
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

    fn static_files_with_paths(paths: &[&str]) -> Vec<Located<StaticFilesSpec>> {
        vec![Located::detached(StaticFilesSpec {
            routes: paths
                .iter()
                .map(|p| {
                    Located::detached(StaticRouteSpec {
                        path: Located::detached(p.to_string()),
                        file_dir: Located::detached(std::env::temp_dir()),
                        ..Default::default()
                    })
                })
                .collect(),
        })]
    }

    #[test]
    fn static_route_path_requires_a_leading_slash() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            static_files: static_files_with_paths(&["assets"]),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "path is not a valid route path: \"assets\""),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn duplicate_static_and_service_route_path_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            services: vec![minimal_service()],
            static_files: static_files_with_paths(&["/"]),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate route path")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn duplicate_static_route_paths_are_rejected() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(minimal_bind())),
            static_files: static_files_with_paths(&["/dup", "/dup"]),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate route path")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn redirect_port_on_a_different_interface_is_allowed() {
        // Arrange
        let mut report = Report::new();
        let make_ingress = |interface: &str, bind_port: i64| {
            let mut bind = minimal_bind();
            bind.interface = Located::detached(interface.to_string());
            bind.port = Located::detached(bind_port);
            bind.tls = Some(Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached("example.com".to_string())],
                challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
            }));
            bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
                port: Located::detached(9090),
                status: Located::detached(308),
            }));
            Located::detached(IngressSpec {
                bind: Some(Located::detached(bind)),
                ..Default::default()
            })
        };

        // Act
        validate_ingresses(
            &[
                make_ingress("127.0.0.1", 8080),
                make_ingress("127.0.0.2", 8081),
            ],
            &mut report,
        );

        // Assert
        assert!(
            !report
                .issues()
                .iter()
                .any(|e| e.message.contains("redirect")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn redirect_port_colliding_with_a_bind_admin_port_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let mut bind = minimal_bind();
        bind.tls = Some(Located::detached(TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
        }));
        bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
            port: Located::detached(9000),
            status: Located::detached(308),
        }));
        let cert = NamedTempFile::new().unwrap();
        let key = NamedTempFile::new().unwrap();
        let token = NamedTempFile::new().unwrap();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(bind)),
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("127.0.0.1".to_string()),
                port: Located::detached(9000),
                tls: Located::detached(TlsTerminationSpec::Manual {
                    cert: Located::detached(cert.path().to_path_buf()),
                    key: Located::detached(key.path().to_path_buf()),
                }),
                auth: Some(Located::detached(AdminAuthSpec {
                    bearer: Some(Located::detached(BearerAuthSpec {
                        token_file: Located::detached(token.path().to_path_buf()),
                    })),
                })),
            })),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert: the shared listener key set catches the collision whichever
        // party registered the address first, so accept either wording.
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate") && e.message.contains("127.0.0.1:9000")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn redirect_port_equal_to_its_own_bind_port_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let mut bind = minimal_bind();
        bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
            port: Located::detached(8080),
            status: Located::detached(308),
        }));
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(bind)),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate redirect listener address")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn services_on_an_admin_only_ingress_warn() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("127.0.0.1".to_string()),
                port: Located::detached(9000),
                auth: Some(Located::detached(AdminAuthSpec::default())),
                ..Default::default()
            })),
            services: vec![minimal_service()],
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("are ignored")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn an_admin_only_ingress_without_services_warns_nothing() {
        // Arrange
        let mut report = Report::new();
        let ingress = Located::detached(IngressSpec {
            bind_admin: Some(Located::detached(BindAdminSpec {
                interface: Located::detached("127.0.0.1".to_string()),
                port: Located::detached(9000),
                auth: Some(Located::detached(AdminAuthSpec::default())),
                ..Default::default()
            })),
            ..Default::default()
        });

        // Act
        validate_ingresses(&[ingress], &mut report);

        // Assert
        assert!(
            !report
                .issues()
                .iter()
                .any(|e| e.message.contains("are ignored")),
            "issues: {:?}",
            report.issues()
        );
    }
}
