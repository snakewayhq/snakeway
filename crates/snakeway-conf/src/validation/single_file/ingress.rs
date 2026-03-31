use crate::types::{BindInterfaceInput, BindInterfaceSpec, IngressSpec, Origin};
use crate::validation::ValidationReport;
use std::collections::HashSet;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub(crate) fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
    let mut seen_listener_keys = HashSet::new();
    let mut seen_redirect_ports = HashSet::new();
    let mut seen_upstream_socks = HashSet::new();

    for ingress in ingresses {
        //---------------------------------------------------------------------
        // Validate listener uniqueness.
        //---------------------------------------------------------------------
        let maybe_bind = ingress.bind.as_ref();
        if let Some(bind) = maybe_bind {
            validate_listener_uniqueness(
                &bind.interface,
                bind.port,
                &bind.origin,
                report,
                &mut seen_listener_keys,
            );
        }

        let maybe_bind_admin = ingress.bind_admin.as_ref();
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
        // Bind validation.
        //---------------------------------------------------------------------
        if let Some(bind) = &ingress.bind {
            if let Some(redirect) = &bind.redirect_http_to_https
                && !seen_redirect_ports.insert(redirect.port)
            {
                report.duplicate_redirect_http_to_https_port(redirect.port, &bind.origin);
            }
        }

        // There must be at least one bind or admin bind.
        if ingress.bind.is_none() && ingress.bind_admin.is_none() {
            report.missing_bind(&ingress.origin);
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
