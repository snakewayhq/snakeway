use crate::conf::types::{
    BindInterfaceSpec, BindSpec, HostSpec, IngressSpec, Origin, RedirectSpec, ServiceSpec,
    StaticFilesSpec,
};
use crate::conf::validation::ValidationReport;
use crate::conf::validation::validator::{
    CB_FAILURE_THRESHOLD, CB_HALF_OPEN_MAX_REQUESTS, CB_OPEN_DURATION_MS, CB_SUCCESS_THRESHOLD,
    REDIRECT_RESPONSE_CODE, is_valid_hostname, is_valid_port, validate_range,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Validate listener definitions.
///
/// Structural errors here are aggregated, not fail-fast.
pub fn validate_ingresses(ingresses: &[IngressSpec], report: &mut ValidationReport) {
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

            if let Some(tls) = &bind.tls {
                if !Path::new(&tls.cert).is_file() {
                    report.missing_cert_file(&tls.cert, &bind.origin);
                }
                if !Path::new(&tls.key).is_file() {
                    report.missing_key_file(&tls.key, &bind.origin);
                }
            }

            // HTTP/2 requires TLS
            if bind.enable_http2 && bind.tls.is_none() {
                report.http2_requires_tls(&bind.interface.to_string(), &bind.origin);
            }

            if let Some(redirect) = &bind.redirect_http_to_https {
                validate_redirect(redirect, &bind.origin, report);

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
                if let Some(sock) = &upstream.sock {
                    if !seen_upstream_socks.insert(sock.clone()) {
                        report.duplicate_upstream_sock(sock, &service.origin);
                    }
                }
            }
        }
    }
}

/// Validate Static files
fn validate_static_files(static_file_specs: &[StaticFilesSpec], report: &mut ValidationReport) {
    for spec in static_file_specs {
        for route in &spec.routes {
            if !route.file_dir.exists() {
                report.invalid_static_dir(&route.file_dir, &route.origin);
            }
            if route.file_dir.is_relative() {
                report.invalid_static_dir_must_be_absolute(&route.file_dir, &route.origin);
            }
        }
    }
}

/// Validate redirect configuration.
pub fn validate_redirect(spec: &RedirectSpec, origin: &Origin, report: &mut ValidationReport) {
    if !is_valid_port(spec.port) {
        report.invalid_port(spec.port, origin);
    }

    validate_range(spec.status, &REDIRECT_RESPONSE_CODE, report, origin);
}

/// Validate service definitions.
pub fn validate_services(
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
            if bind_uses_http2 && route.enable_websocket {
                report.websocket_route_cannot_be_used_with_http2(&route.path, &route.origin);
            }
        }

        // Upstreams
        for upstream in &service.upstreams {
            if upstream.weight == 0 || upstream.weight > 1_000 {
                report.invalid_upstream_weight(&upstream.weight, &service.origin);
            }

            let has_sock = upstream.sock.is_some();

            let has_endpoint = upstream
                .endpoint
                .as_ref()
                .is_some_and(|endpoint| is_valid_port(endpoint.port));

            match (has_endpoint, has_sock) {
                (true, false) => {} // valid: tcp upstream
                (false, true) => {} // valid: unix upstream
                _ => {
                    // both or neither (or incomplete endpoint)
                    report.invalid_upstream_target(&service.origin);
                    continue;
                }
            }

            if let Some(endpoint) = &upstream.endpoint {
                match &endpoint.host {
                    HostSpec::Ip(ip) if ip.is_unspecified() || ip.is_multicast() => {
                        report.invalid_upstream_ip(ip, &service.origin);
                    }
                    HostSpec::Hostname(name) if !is_valid_hostname(name) => {
                        report.invalid_upstream_hostname(name, &service.origin);
                    }
                    _ => {}
                }

                if !is_valid_port(endpoint.port) {
                    report.invalid_port(endpoint.port, &service.origin);
                }
            }

            if let Some(sock) = &upstream.sock
                && seen_sock_values.insert(sock.clone(), ()).is_some()
            {
                report.duplicate_upstream_sock(sock, &service.origin);
            }
        }

        // Circuit breaker
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
