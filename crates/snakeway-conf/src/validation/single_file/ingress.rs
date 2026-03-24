use crate::types::{BindInterfaceSpec, BindSpec, IngressSpec, ServiceSpec, StaticFilesSpec};
#[cfg(test)]
use crate::types::{Origin, RedirectSpec};
use crate::validation::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
use crate::validation::validator::is_valid_port;
use std::collections::{HashMap, HashSet};

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub(crate) fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
    let mut seen_listener_keys = HashSet::new();
    let mut seen_redirect_ports = HashSet::new();
    let mut seen_upstream_socks = HashSet::new();

    for ingress in ingresses {
        // ---------------------------------------------------------------------
        // Bind
        // ---------------------------------------------------------------------
        if let Some(bind) = &ingress.bind {
            if !is_valid_port(bind.port) {
                report.invalid_port(bind.port, &bind.origin);
            }

            if let Some(connection_filter) = &bind.connection_filter {
                connection_filter.validate(&bind.origin, report);
            }

            if let Some(connection_rate_limiting_filter) = &bind.connection_rate_limiting_filter {
                connection_rate_limiting_filter.validate(&bind.origin, report);
            }

            let interface: Result<BindInterfaceSpec, _> = bind.interface.clone().try_into();
            match interface {
                Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                    report.invalid_bind_addr("0.0.0.0", &bind.origin);
                }
                Ok(spec) => {
                    let key = format!("{}:{}", spec.as_ip(), bind.port);
                    if !seen_listener_keys.insert(key.clone()) {
                        report.duplicate_bind_addr(&key, &bind.origin);
                    }
                }
                Err(_) => {
                    report.invalid_bind_addr(&bind.interface.to_string(), &bind.origin);
                }
            }

            if let Some(certificate_spec) = &bind.tls {
                certificate_spec.validate(&bind.origin, report);
            }

            // HTTP/2 requires TLS
            if bind.enable_http2 && bind.tls.is_none() {
                report.http2_requires_tls(&bind.interface.to_string(), &bind.origin);
            }

            if let Some(redirect) = &bind.redirect_http_to_https {
                redirect.validate(&bind.origin, report);

                if bind.tls.is_none() {
                    report.redirect_http_to_https_requires_tls(
                        &bind.interface.to_string(),
                        &bind.origin,
                    );
                }

                if !seen_redirect_ports.insert(redirect.port) {
                    report.duplicate_redirect_http_to_https_port(redirect.port, &bind.origin);
                }
            }
        }

        // ---------------------------------------------------------------------
        // Admin bind
        // ---------------------------------------------------------------------
        if let Some(bind_admin) = &ingress.bind_admin {
            if !is_valid_port(bind_admin.port) {
                report.invalid_port(bind_admin.port, &bind_admin.origin);
            }

            let interface: Result<BindInterfaceSpec, _> = bind_admin.interface.clone().try_into();
            match interface {
                Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                    report.invalid_bind_addr("0.0.0.0", &bind_admin.origin);
                }
                Ok(spec) => {
                    let key = format!("{}:{}", spec.as_ip(), bind_admin.port);
                    if !seen_listener_keys.insert(key.clone()) {
                        report.duplicate_bind_addr(&key, &bind_admin.origin);
                    }
                }
                Err(_) => {
                    report.invalid_bind_addr(&bind_admin.interface.to_string(), &bind_admin.origin);
                }
            }

            // Guard against binding the admin API to all interfaces.
            // This is a dangerous situation because the admin API does not currently have
            // authentication and could be used to gain unauthorized access to the server.
            let bind_interface: BindInterfaceSpec = match bind_admin.interface.clone().try_into() {
                Ok(i) => i,
                Err(_) => {
                    report.invalid_bind_addr(&bind_admin.interface.to_string(), &bind_admin.origin);
                    continue;
                }
            };

            if matches!(bind_interface, BindInterfaceSpec::All) {
                report.error(
                    "admin API cannot bind to all interfaces".to_string(),
                    &bind_admin.origin,
                    Some("Use loopback or a specific IP address.".to_string()),
                );
            }

            match &bind_admin.tls {
                tls @ crate::types::TlsTerminationSpec::Manual { .. } => {
                    tls.validate(&bind_admin.origin, report);
                }
                crate::types::TlsTerminationSpec::Acme { .. } => {
                    report.admin_bind_does_not_support_acme(&bind_admin.origin);
                }
            }
        }

        if ingress.bind.is_none() && ingress.bind_admin.is_none() {
            report.missing_bind(&ingress.origin);
        }

        validate_static_files(&ingress.static_files, report);
        validate_services(&ingress.bind, &ingress.services, report);

        // ---------------------------------------------------------------------
        // Cross-ingress upstream sock uniqueness
        // ---------------------------------------------------------------------
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
/// Validate Static files
fn validate_static_files(static_file_specs: &[StaticFilesSpec], report: &mut ValidationReport) {
    for spec in static_file_specs {
        for route in &spec.routes {
            route.validate(&route.origin, report);
        }
    }
}

/// Validate service definitions.
pub(crate) fn validate_services(
    maybe_bind: &Option<BindSpec>,
    services: &[ServiceSpec],
    report: &mut ValidationReport,
) {
    let bind_uses_http2 = maybe_bind.as_ref().is_some_and(|b| b.enable_http2);

    for service in services {
        if service.upstreams.is_empty() {
            report.service_has_no_upstreams(&service.origin);
        }

        let mut seen_sock_values = HashMap::new();

        // Routes
        for route in &service.routes {
            if route.hosts.is_empty() {
                report.route_has_no_hosts(&service.origin);
            }

            if bind_uses_http2 && route.enable_websocket {
                report.websocket_route_cannot_be_used_with_http2(&route.path, &route.origin);
            }
        }

        // Upstreams
        for upstream in &service.upstreams {
            upstream.validate(&service.origin, report);

            if let (Some(sock), Some(endpoint)) = (&upstream.sock, &upstream.endpoint) {
                report.upstream_cannot_have_both_sock_and_endpoint(
                    sock,
                    &endpoint.host.to_string(),
                    endpoint.port,
                    &service.origin,
                );
                continue;
            }

            if upstream.sock.is_none() && upstream.endpoint.is_none() {
                report.upstream_must_have_a_sock_or_endpoint(&service.origin);
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
                && seen_sock_values.insert(sock.clone(), ()).is_some()
            {
                report.duplicate_upstream_sock(sock, &upstream.origin);
            }
        }

        // Circuit breaker
        if let Some(cb) = &service.circuit_breaker
            && cb.enable_auto_recovery
        {
            cb.validate(&service.origin, report);
        }
    }
}

/// Validate redirect configuration.
///
/// Delegates field-local validation to the `ValidateSpec` trait implementation
/// on `RedirectSpec`. Retained for test compatibility.
#[cfg(test)]
pub(crate) fn validate_redirect(
    spec: &RedirectSpec,
    origin: &Origin,
    report: &mut ValidationReport,
) {
    spec.validate(origin, report);
}
