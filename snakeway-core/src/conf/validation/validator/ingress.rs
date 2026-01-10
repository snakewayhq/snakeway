use crate::conf::types::{ExposeServiceConfig, IngressConfig};
use crate::conf::validation::ValidationReport;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::Path;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub fn validate_ingresses(ingresses: &[IngressConfig], report: &mut ValidationReport) {
    let mut seen_listener_addrs = HashSet::new();

    for ingress in ingresses {
        // Bind
        if let Some(bind) = &ingress.bind {
            let bind_addr: SocketAddr = match bind.addr.parse() {
                Ok(a) => a,
                Err(_) => {
                    report.error(
                        format!("invalid listener address: {}", bind.addr),
                        &bind.origin,
                        None,
                    );
                    continue;
                }
            };

            if bind_addr.ip().is_unspecified() {
                report.error(
                    "invalid listener address: unspecified IP address".to_string(),
                    &bind.origin,
                    None,
                );
            }

            if !seen_listener_addrs.insert(&bind.addr) {
                report.error(
                    format!("duplicate listener address: {}", bind.addr),
                    &bind.origin,
                    None,
                );
            }

            if let Some(tls) = &bind.tls {
                if !Path::new(&tls.cert).is_file() {
                    report.error(
                        format!("missing certificate file: {}", tls.cert),
                        &bind.origin,
                        None,
                    );
                }
                if !Path::new(&tls.key).is_file() {
                    report.error(format!("missing key file: {}", tls.key), &bind.origin, None);
                }
            }

            // HTTP/2 requires TLS.
            if bind.enable_http2 && bind.tls.is_none() {
                report.error(
                    format!("HTTP/2 listener requires TLS: {}", bind.addr),
                    &bind.origin,
                    None,
                );
            }
        }

        if let Some(bind_admin) = &ingress.bind_admin {
            if !seen_listener_addrs.insert(&bind_admin.addr) {
                report.error(
                    format!("duplicate listener address: {}", bind_admin.addr),
                    &bind_admin.origin,
                    None,
                );
            }

            let bind_admin_addr: SocketAddr = match bind_admin.addr.parse() {
                Ok(a) => a,
                Err(_) => {
                    report.error(
                        format!("invalid listener address: {}", bind_admin.addr),
                        &bind_admin.origin,
                        None,
                    );
                    continue;
                }
            };

            if bind_admin_addr.ip().is_unspecified() {
                report.error(
                    "invalid listener address: unspecified IP address".to_string(),
                    &bind_admin.origin,
                    None,
                );
            }
        }

        validate_services(&ingress.service_cfgs, report);

        // Static Files
        for cfg in ingress.static_cfgs.iter() {
            for route in cfg.routes.iter() {
                if !route.file_dir.exists() {
                    report.error(
                        format!("invalid static directory: {}", route.file_dir.display()),
                        &route.origin,
                        None,
                    );
                }
            }
        }
    }
}

/// Validate service definitions.
pub fn validate_services(services: &[ExposeServiceConfig], report: &mut ValidationReport) {
    for service in services {
        if service.backends.is_empty() {
            report.error(
                "service has no upstreams".to_string(),
                &service.origin,
                None,
            );
            continue;
        }

        let mut seen_sock_values = HashMap::new();

        // Validate backends
        for backend in &service.backends {
            if backend.weight == 0 || backend.weight > 1_000 {
                report.error(
                    format!(
                        "invalid upstream weight - must be between 1 and 1000 (inclusive): {}",
                        backend.weight
                    ),
                    &service.origin,
                    None,
                );
            }

            let backend_addr = backend.addr.clone().unwrap_or_default();
            let backend_sock = backend.sock.clone().unwrap_or_default();
            let neither_are_defined = backend_addr.is_empty() && backend_sock.is_empty();
            let both_are_defined = !backend_addr.is_empty() && !backend_sock.is_empty();
            if neither_are_defined || both_are_defined {
                report.error(
                    format!(
                        "invalid upstream - addr (TCP) or a sock (UNIX) are mutually exclusive: {} {}",
                        backend.addr.as_ref().unwrap(),
                        backend.sock.as_ref().unwrap(),
                    ),
                    &service.origin,
                    None,
                );
                continue;
            }

            if let Some(addr) = &backend.addr {
                if addr.parse::<SocketAddr>().is_err() {
                    report.error(
                        format!("invalid upstream - invalid addr: {}", addr),
                        &service.origin,
                        None,
                    );
                }
            }

            // Duplicate socks?
            if let Some(sock) = &backend.sock {
                if seen_sock_values.contains_key(sock) {
                    report.error(
                        format!("invalid upstream - duplicate socket path: {}", sock),
                        &service.origin,
                        None,
                    );
                } else {
                    seen_sock_values.insert(sock.clone(), ());
                }

                if let Some(sock_options) = &backend.sock_options {
                    if sock_options.use_tls && sock_options.sni.is_empty() {
                        report.error(
                            format!(
                                "invalid upstream - SNI must be set when using TLS: {}",
                                sock
                            ),
                            &service.origin,
                            None,
                        );
                    }
                }
            }
        }

        // Validate circuit breaker
        if let Some(cb) = &service.circuit_breaker
            && cb.enable_auto_recovery
            && (cb.failure_threshold == 0
                || cb.open_duration_milliseconds == 0
                || cb.half_open_max_requests == 0
                || cb.success_threshold == 0)
        {
            report.error(
                "invalid circuit breaker - all thresholds must be >= 1".to_string(),
                &service.origin,
                None,
            );
        }
    }
}
