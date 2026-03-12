use crate::types::{DeviceSpec, Origin};
use crate::validation::ValidationReport;
use crate::validation::validator::{
    IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH, IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
    REQUEST_FILTER_DENY_STATUS, REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
    REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS, validate_http_header_name, validate_http_method,
    validate_range,
};
use ipnet::IpNet;
use nix::NixPath;
use std::net::IpAddr;
use std::path::Path;

pub(crate) fn validate_devices(devices: &[DeviceSpec], report: &mut ValidationReport) {
    let mut identity_seen = false;
    let mut identity_enabled = false;
    let mut network_policy_seen = false;
    let mut request_rate_limiting_device_seen = false;
    let mut request_filter_seen = false;
    let mut structured_logging_seen = false;

    // Validate identity device spec first.
    let enabled_devices = devices.iter().filter(|device| device.is_enabled());
    for device in enabled_devices {
        if let DeviceSpec::Identity(cfg) = device {
            if identity_seen {
                report.device_already_defined(device.origin());
            }
            identity_seen = true;
            identity_enabled = cfg.enable;

            validate_trusted_proxies(&cfg.trusted_proxies, report, device.origin());
            validate_range(
                cfg.max_x_forwarded_for_length,
                &IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
                report,
                device.origin(),
            );

            if cfg.enable_user_agent {
                validate_range(
                    cfg.max_user_agent_length,
                    &IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH,
                    report,
                    device.origin(),
                );
            }

            if cfg.enable_geoip {
                if cfg.geoip_city_db.is_none()
                    && cfg.geoip_isp_db.is_none()
                    && cfg.geoip_connection_type_db.is_none()
                {
                    report.geoip_enabled_with_no_dbs_specified(device.origin());
                }

                if let Some(path) = cfg.geoip_city_db.as_ref() {
                    validate_geoip_db_file(path, report, device.origin());
                }

                if let Some(path) = cfg.geoip_isp_db.as_ref() {
                    validate_geoip_db_file(path, report, device.origin());
                }

                if let Some(geoip_city_db) = cfg.geoip_connection_type_db.as_ref() {
                    validate_geoip_db_file(geoip_city_db, report, device.origin());
                }
            }
        };
    }

    // Validate remaining device specs (some of which may depend on the presence of the identity).
    let enabled_devices = devices.iter().filter(|device| device.is_enabled());
    for device in enabled_devices {
        match device {
            DeviceSpec::RequestFilter(cfg) => {
                if request_filter_seen {
                    report.device_already_defined(device.origin());
                }
                request_filter_seen = true;

                if let Some(deny_status) = cfg.deny_status {
                    validate_range(
                        deny_status,
                        &REQUEST_FILTER_DENY_STATUS,
                        report,
                        device.origin(),
                    );
                }

                for method in &cfg.allow_methods {
                    validate_http_method(method, report, device.origin());
                }

                for method in &cfg.deny_methods {
                    validate_http_method(method, report, device.origin());
                }

                for header in &cfg.allow_headers {
                    validate_http_header_name(header, report, device.origin());
                }

                for header in &cfg.allow_headers {
                    validate_http_header_name(header, report, device.origin());
                }

                for header in &cfg.allow_headers {
                    validate_http_header_name(header, report, device.origin());
                }

                if cfg.max_suspicious_body_bytes > cfg.max_body_bytes {
                    report.warn_max_suspicious_bytes_large_than_max_body_bytes(device.origin());
                }
            }
            DeviceSpec::NetworkPolicy(cfg) => {
                if network_policy_seen {
                    report.device_already_defined(device.origin());
                }
                network_policy_seen = true;

                if !identity_enabled {
                    // The network policy device requires the identity device to be present.
                    // It is a no-op internally if the identity device is not present, but it is
                    // import to validate its presence here to a void network policy silently
                    // being ignored.
                    report.device_requires_identity_device(device.origin());
                }

                if cfg.cidr_allow.is_empty() {
                    report.network_policy_device_requires_cidr_allow(device.origin());
                }

                for cidr in &cfg.cidr_allow {
                    if cidr.parse::<IpNet>().is_err() {
                        report.invalid_network_policy_cidr(cidr, device.origin());
                    }
                }
            }
            DeviceSpec::RequestRateLimiting(cfg) => {
                if request_rate_limiting_device_seen {
                    report.device_already_defined(device.origin());
                }
                request_rate_limiting_device_seen = true;

                if !identity_enabled {
                    // The request rate limiting device requires the identity device to be present.
                    // It is a no-op internally if the identity device is not present, but it is
                    // import to validate its presence here to a void request rate limiting silently
                    // being ignored.
                    report.device_requires_identity_device(device.origin());
                }

                validate_range(
                    cfg.max_requests_per_second,
                    &REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
                    report,
                    device.origin(),
                );
                validate_range(
                    cfg.window_seconds,
                    &REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS,
                    report,
                    device.origin(),
                );
            }
            DeviceSpec::Wasm(cfg) => {
                if cfg.path.is_empty() {
                    report.wasm_device_path_is_empty(cfg.path.display(), device.origin());
                }
                if !cfg.path.exists() {
                    report.wasm_device_path_does_not_exist(cfg.path.display(), device.origin());
                }
                if !cfg.path.is_file() {
                    report.wasm_device_path_is_not_a_file(cfg.path.display(), device.origin());
                }
            }
            DeviceSpec::StructuredLogging(cfg) => {
                if structured_logging_seen {
                    report.device_already_defined(device.origin());
                }
                structured_logging_seen = true;

                if cfg.include_identity && cfg.identity_fields.is_empty() {
                    report.structured_logging_identity_fields_empty(device.origin());
                }

                if cfg.include_headers
                    && cfg.allowed_headers.is_empty()
                    && cfg.redacted_headers.is_empty()
                {
                    report.structured_logging_includes_headers_but_no_headers_set(device.origin());
                }
            }
            DeviceSpec::Identity(_) => {
                // No-op, identity device was already validated.
            }
        };
    }
}

fn validate_geoip_db_file(geoip_db: &Path, report: &mut ValidationReport, origin: &Origin) -> bool {
    let mut has_error = false;
    if !geoip_db.is_file() {
        if NixPath::is_empty(geoip_db) {
            report.geoip_db_path_is_empty(geoip_db.display(), origin);
            has_error = true;
        }
        if !geoip_db.exists() {
            report.geoip_db_path_does_not_exist(geoip_db.display(), origin);
            has_error = true;
        }
        if !geoip_db.is_file() {
            report.geoip_db_is_not_a_file(geoip_db.display(), origin);
            has_error = true;
        }
    }
    !has_error
}

fn validate_trusted_proxies(proxies: &[String], report: &mut ValidationReport, origin: &Origin) {
    let mut networks = Vec::new();
    for proxy in proxies {
        if let Ok(net) = proxy.parse::<IpNet>() {
            networks.push(net);
        } else {
            report.invalid_trusted_proxy(proxy, origin);
        }
    }

    for network in networks {
        // Security note: Trusting all proxies is a catastrophic misconfiguration.
        if network.prefix_len() == 0 {
            report.trusted_proxies_cannot_trust_all_networks(origin);
        }

        // Trusting public IP ranges is a red flag/gray area.
        // Some environments must trust public IPs, but they should feel nervous about it.
        if !is_non_public_infra_network(&network) {
            report.trusted_proxies_contains_a_public_ip_range_warning(network, origin);
        }
    }
}

/// NOTE: This function identifies non-globally-routable infrastructure address
/// space (RFC1918, ULA, loopback, link-local).
/// It MUST NOT be used to determine the absolute trustworthiness of a peer.
fn is_non_public_infra_network(net: &IpNet) -> bool {
    match &net.addr() {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}
