use crate::conf::types::{IngressSpec, ServiceSpec};
use crate::conf::validation::ValidationReport;
use crate::conf::validation::validator::range::{
    CB_FAILURE_THRESHOLD, CB_HALF_OPEN_MAX_REQUESTS, CB_OPEN_DURATION_MS, CB_SUCCESS_THRESHOLD,
    validate_range,
};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::Path;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
    let mut seen_listener_addrs = HashSet::new();

    for ingress in ingresses {
        // Bind
        if let Some(bind) = &ingress.bind {
            let bind_addr: Option<SocketAddr> = match bind.addr.parse() {
                Ok(a) => Some(a),
                Err(_) => {
                    report.invalid_bind_addr(&bind.addr, &bind.origin);
                    None
                }
            };

            if let Some(addr) = bind_addr
                && addr.ip().is_unspecified()
            {
                report.invalid_bind_addr(&bind.addr, &bind.origin);
            }

            if !seen_listener_addrs.insert(&bind.addr) {
                report.duplicate_bind_addr(&bind.addr, &bind.origin);
            }

            if let Some(tls) = &bind.tls {
                if !Path::new(&tls.cert).is_file() {
                    report.missing_cert_file(&tls.cert, &bind.origin);
                }
                if !Path::new(&tls.key).is_file() {
                    report.missing_key_file(&tls.key, &bind.origin);
                }
            }

            // HTTP/2 requires TLS.
            if bind.enable_http2 && bind.tls.is_none() {
                report.http2_requires_tls(&bind.addr, &bind.origin);
            }
        }

        if let Some(bind_admin) = &ingress.bind_admin {
            if !seen_listener_addrs.insert(&bind_admin.addr) {
                report.duplicate_bind_addr(&bind_admin.addr, &bind_admin.origin);
            }

            let bind_admin_addr: Option<SocketAddr> = match bind_admin.addr.parse() {
                Ok(a) => Some(a),
                Err(_) => {
                    report.invalid_bind_addr(&bind_admin.addr, &bind_admin.origin);
                    None
                }
            };

            if let Some(addr) = bind_admin_addr
                && addr.ip().is_unspecified()
            {
                report.invalid_bind_addr(&bind_admin.addr, &bind_admin.origin);
            }
        }

        validate_services(&ingress.service_cfgs, report);

        // Static Files
        for cfg in ingress.static_cfgs.iter() {
            for route in cfg.routes.iter() {
                if !route.file_dir.exists() {
                    report.invalid_static_dir(&route.file_dir, &route.origin);
                }
            }
        }
    }
}

/// Validate service definitions.
pub fn validate_services(services: &[ServiceSpec], report: &mut ValidationReport) {
    for service in services {
        if service.upstreams.is_empty() {
            report.service_has_no_upstreams(&service.origin);
        }

        let mut seen_sock_values = HashMap::new();

        // Validate upstreams
        for upstream in &service.upstreams {
            if upstream.weight == 0 || upstream.weight > 1_000 {
                report.invalid_upstream_weight(&upstream.weight, &service.origin);
            }

            let backend_addr = upstream.addr.clone().unwrap_or_default();
            let backend_sock = upstream.sock.clone().unwrap_or_default();
            let neither_are_defined = backend_addr.is_empty() && backend_sock.is_empty();
            let both_are_defined = !backend_addr.is_empty() && !backend_sock.is_empty();
            if neither_are_defined || both_are_defined {
                report.invalid_upstream_target(&upstream.addr, &upstream.sock, &service.origin);
                continue;
            }

            if let Some(addr) = &upstream.addr
                && addr.parse::<SocketAddr>().is_err()
            {
                report.invalid_upstream_addr(&upstream.addr, &service.origin);
            }

            // Duplicate socks?
            if let Some(sock) = &upstream.sock {
                if seen_sock_values.contains_key(sock) {
                    report.duplicate_upstream_sock(sock, &service.origin);
                } else {
                    seen_sock_values.insert(sock.clone(), ());
                }

                if let Some(sock_options) = &upstream.sock_options
                    && sock_options.use_tls
                    && sock_options.sni.is_empty()
                {
                    report.invalid_sock_options_tls_requires_sni(&service.origin);
                }
            }
        }

        // Validate circuit breaker
        if let Some(cb) = &service.circuit_breaker
            && cb.enable_auto_recovery
        {
            validate_range(
                cb.failure_threshold,
                &CB_FAILURE_THRESHOLD,
                report,
                &service.origin,
            );

            validate_range(
                cb.open_duration_milliseconds,
                &CB_OPEN_DURATION_MS,
                report,
                &service.origin,
            );

            validate_range(
                cb.half_open_max_requests,
                &CB_HALF_OPEN_MAX_REQUESTS,
                report,
                &service.origin,
            );

            validate_range(
                cb.success_threshold,
                &CB_SUCCESS_THRESHOLD,
                report,
                &service.origin,
            );
        }
    }
}
