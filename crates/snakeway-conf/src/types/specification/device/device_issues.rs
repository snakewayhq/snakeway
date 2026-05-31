use crate::types::HclOrigin;
use confval::ValidationIssue;
use std::path::Display;

pub(crate) fn device_already_defined(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("device already defined", origin.clone())
}

pub(crate) fn device_requires_identity_device(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "device requires identity device to be present and enabled",
        origin.clone(),
    )
}

pub(crate) fn device_path_must_start_with_slash(
    path: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("device path must start with '/': {path}"),
        origin.clone(),
    )
}

pub(crate) fn network_policy_device_requires_cidr_allow(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "network policy device requires cidr_allow list to be set",
        origin.clone(),
    )
}

pub(crate) fn invalid_network_policy_cidr(
    cidr: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid network policy CIDR: {}", cidr),
        origin.clone(),
    )
}

pub(crate) fn wasm_device_name_is_empty(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("wasm device name must not be empty", origin.clone())
}

pub(crate) fn wasm_device_duplicate_name(
    name: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("duplicate wasm device name: \"{}\"", name),
        origin.clone(),
    )
}

pub(crate) fn wasm_device_path_is_empty(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("wasm device path is empty: {}", path),
        origin.clone(),
    )
}

pub(crate) fn wasm_device_path_does_not_exist(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("wasm device path does not exist: {}", path),
        origin.clone(),
    )
}

pub(crate) fn wasm_device_path_is_not_a_file(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("wasm device path is not a file: {}", path),
        origin.clone(),
    )
}

pub(crate) fn geoip_enabled_with_no_dbs_specified(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning_with_help(
        "geoip enabled with no dbs specified",
        origin.clone(),
        "At least one geoip db must be specified",
    )
}

pub(crate) fn geoip_db_path_is_empty(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("geoip db path is empty: {}", path), origin.clone())
}

pub(crate) fn geoip_db_path_does_not_exist(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("geoip db path does not exist: {}", path),
        origin.clone(),
    )
}

pub(crate) fn geoip_db_is_not_a_file(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("geoip db path is not a file: {}", path),
        origin.clone(),
    )
}

pub(crate) fn ua_parser_regexes_path_is_empty(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("ua_parser_regexes path is empty: {}", path),
        origin.clone(),
    )
}

pub(crate) fn ua_parser_regexes_path_does_not_exist(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("ua_parser_regexes path does not exist: {}", path),
        origin.clone(),
        "Provide a valid path to a ua-parser regexes.yaml file, or remove the setting to use the bundled default.",
    )
}

pub(crate) fn ua_parser_regexes_path_is_not_a_file(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("ua_parser_regexes path is not a file: {}", path),
        origin.clone(),
    )
}

pub(crate) fn ua_parser_regexes_file_missing_expected_content(
    path: Display,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning_with_help(
        format!(
            "ua_parser_regexes file does not appear to be a valid ua-parser regexes.yaml: {}",
            path
        ),
        origin.clone(),
        "Expected the file to contain a 'user_agent_parsers' section. See https://github.com/ua-parser/uap-core for the expected format.",
    )
}

pub(crate) fn invalid_trusted_proxy(proxy: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid trusted proxy: {}", proxy), origin.clone())
}

pub(crate) fn trusted_proxies_cannot_trust_all_networks(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        "trusted_proxies must not contain a catch-all network (0.0.0.0/0 or ::/0)",
        origin.clone(),
    )
}

pub(crate) fn trusted_proxies_contains_a_public_ip_range_warning(
    network: ipnet::IpNet,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning(
        format!("trusted_proxies should NOT contain a public IP range: {network}"),
        origin.clone(),
    )
}

pub(crate) fn structured_logging_identity_fields_empty(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning(
        "structured logging identity fields cannot be empty",
        origin.clone(),
    )
}

pub(crate) fn structured_logging_includes_headers_but_no_headers_set(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning_with_help(
        "structured logging includes headers but no headers are set",
        origin.clone(),
        "Add headers to allowed_headers or redacted_headers to include headers in structured logs.",
    )
}

pub(crate) fn invalid_http_method(method: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid HTTP method: {}", method), origin.clone())
}

pub(crate) fn invalid_http_header_name(
    header: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid HTTP header name: {}", header),
        origin.clone(),
    )
}

pub(crate) fn warn_max_suspicious_bytes_large_than_max_body_bytes(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::warning_with_help(
        "max_suspicious_body_bytes should not be larger than max_body_bytes",
        origin.clone(),
        "max_suspicious_body_bytes applies to functions that can technically have a body, but should be treated suspiciously (and thus have a lower max size than a regular body)",
    )
}
