use crate::types::{BindInterfaceInput, BindInterfaceSpec, IngressSpec, Origin};
use crate::validation::{ValidateSpec, ValidationReport};
use std::collections::HashSet;

pub(crate) fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
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
    origin: &Origin,
    report: &mut ValidationReport,
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
