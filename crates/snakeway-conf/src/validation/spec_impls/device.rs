use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, Origin, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, WasmDeviceSpec,
};
use crate::validation::report::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
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

impl ValidateSpec for IdentityDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_trusted_proxies(&self.trusted_proxies, report, origin);

        validate_range(
            self.max_x_forwarded_for_length,
            &IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
            report,
            origin,
        );

        if self.enable_user_agent {
            validate_range(
                self.max_user_agent_length,
                &IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH,
                report,
                origin,
            );
        }

        if self.enable_geoip {
            if let Some(path) = self.geoip_city_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_isp_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_connection_type_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }
        }

        if let Some(path) = self.ua_parser_regexes.as_ref() {
            validate_ua_parser_regexes_file(path, report, origin);
        }
    }
}

impl ValidateSpec for RequestFilterDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if let Some(deny_status) = self.deny_status {
            validate_range(deny_status, &REQUEST_FILTER_DENY_STATUS, report, origin);
        }

        for method in &self.allow_methods {
            validate_http_method(method, report, origin);
        }

        for method in &self.deny_methods {
            validate_http_method(method, report, origin);
        }

        for header in &self.deny_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.allow_headers {
            validate_http_header_name(header, report, origin);
        }

        for header in &self.required_headers {
            validate_http_header_name(header, report, origin);
        }
    }
}

impl ValidateSpec for NetworkPolicyDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        for cidr in &self.cidr_allow {
            if cidr.parse::<IpNet>().is_err() {
                report.invalid_network_policy_cidr(cidr, origin);
            }
        }
    }
}

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.max_requests_per_second,
            &REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
            report,
            origin,
        );
        validate_range(
            self.window_seconds,
            &REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS,
            report,
            origin,
        );
    }
}

impl ValidateSpec for WasmDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.path.is_empty() {
            report.wasm_device_path_is_empty(self.path.display(), origin);
        }
        if !self.path.exists() {
            report.wasm_device_path_does_not_exist(self.path.display(), origin);
        }
        if !self.path.is_file() {
            report.wasm_device_path_is_not_a_file(self.path.display(), origin);
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions (moved from single_file/device.rs)
// ---------------------------------------------------------------------------

fn validate_geoip_db_file(geoip_db: &Path, report: &mut ValidationReport, origin: &Origin) {
    if !geoip_db.is_file() {
        if NixPath::is_empty(geoip_db) {
            report.geoip_db_path_is_empty(geoip_db.display(), origin);
        }
        if !geoip_db.exists() {
            report.geoip_db_path_does_not_exist(geoip_db.display(), origin);
        }
        if !geoip_db.is_file() {
            report.geoip_db_is_not_a_file(geoip_db.display(), origin);
        }
    }
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
        if network.prefix_len() == 0 {
            report.trusted_proxies_cannot_trust_all_networks(origin);
        }

        if !is_non_public_infra_network(&network) {
            report.trusted_proxies_contains_a_public_ip_range_warning(network, origin);
        }
    }
}

fn validate_ua_parser_regexes_file(path: &Path, report: &mut ValidationReport, origin: &Origin) {
    if NixPath::is_empty(path) {
        report.ua_parser_regexes_path_is_empty(path.display(), origin);
        return;
    }
    if !path.exists() {
        report.ua_parser_regexes_path_does_not_exist(path.display(), origin);
        return;
    }
    if !path.is_file() {
        report.ua_parser_regexes_path_is_not_a_file(path.display(), origin);
        return;
    }
    if let Ok(contents) = std::fs::read_to_string(path)
        && !contents.contains("user_agent_parsers")
    {
        report.ua_parser_regexes_file_missing_expected_content(path.display(), origin);
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
