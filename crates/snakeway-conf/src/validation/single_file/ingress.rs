use crate::types::{BindInterfaceSpec, IngressSpec, TlsTerminationSpec};
use crate::validation::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
use std::collections::HashSet;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub(crate) fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
    let mut seen_listener_keys = HashSet::new();
    let mut seen_redirect_ports = HashSet::new();
    let mut seen_upstream_socks = HashSet::new();

    //---------------------------------------------------------------------
    // Validate listener uniqueness.
    //---------------------------------------------------------------------
    ingresses.iter().for_each(|ingress| {
        let maybe_bind = ingress.bind.as_ref();
        if let (Some(bind)) = maybe_bind {
            if let (Ok(interface)) = bind.interface.clone().try_into() {
                let key = format!("{}:{}", interface.as_ip(), bind.port);
                if !seen_listener_keys.insert(key.clone()) {
                    report.duplicate_bind_addr(&key, &bind.origin);
                }
            }
        }

        let maybe_bind_admin = ingress.bind_admin.as_ref();
        if let (Some(bind_admin)) = maybe_bind_admin {
            if let (Ok(interface)) = bind_admin.interface.clone().try_into() {
                let key = format!("{}:{}", interface.as_ip(), bind_admin.port);
                if !seen_listener_keys.insert(key.clone()) {
                    report.duplicate_bind_admin_addr(&key, &bind_admin.origin);
                }
            }
        }
    });

    for ingress in ingresses {
        //---------------------------------------------------------------------
        // Bind
        //---------------------------------------------------------------------
        if let Some(bind) = &ingress.bind {
            bind.validate(&bind.origin, report);
            if let Some(redirect) = &bind.redirect_http_to_https
                && !seen_redirect_ports.insert(redirect.port)
            {
                report.duplicate_redirect_http_to_https_port(redirect.port, &bind.origin);
            }
        }

        // Admin bind validation.
        if let Some(bind_admin) = &ingress.bind_admin {
            bind_admin.validate(&bind_admin.origin, report);
        }

        // There must be at least one bind or admin bind.
        if ingress.bind.is_none() && ingress.bind_admin.is_none() {
            report.missing_bind(&ingress.origin);
        }

        // Validate Static files
        ingress
            .static_files
            .iter()
            .for_each(|static_files| static_files.validate(&ingress.origin, report));

        let bind_uses_http2 = ingress.bind.as_ref().is_some_and(|b| b.enable_http2);
        ingress
            .services
            .iter()
            .for_each(|service| service.validate(&service.origin, report));

        //---------------------------------------------------------------------
        // Bind/Route http2/websocket agreement.
        // If bind has http2 enabled, websocket routes cannot be used.
        //---------------------------------------------------------------------
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
