use crate::types::HclOrigin;
use confval::ValidationIssue;

pub(crate) fn invalid_bind_addr(addr: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid bind address: {}", addr), origin.clone())
}

pub(crate) fn duplicate_bind_addr(addr: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("duplicate bind address: {}", addr), origin.clone())
}

pub(crate) fn invalid_port(port: i64, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("invalid port: {}", port),
        origin.clone(),
        "ports must be in the range 1–65535",
    )
}

pub(crate) fn http2_requires_tls(addr: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("HTTP/2 requires TLS: {}", addr),
        origin.clone(),
        "Enable TLS on the bind or disable HTTP/2.",
    )
}

pub(crate) fn redirect_http_to_https_requires_tls(
    addr: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("redirect_http_to_https requires TLS: {}", addr),
        origin.clone(),
        "Enable TLS on the bind or remove redirect_http_to_https.",
    )
}

pub(crate) fn duplicate_redirect_http_to_https_port(
    port: i64,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("duplicate redirect_http_to_https port: {}", port),
        origin.clone(),
    )
}

pub(crate) fn ingress_tls_manual_cert_pair_invalid(
    message: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("invalid TLS manual cert pair: {}", message),
        origin.clone(),
        "Use manual mode instead",
    )
}

pub(crate) fn acme_tls_requires_domains(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("missing domains for ACME TLS", origin.clone())
}

pub(crate) fn admin_bind_does_not_support_acme(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("admin bind does not support ACME TLS", origin.clone())
}

pub(crate) fn connection_filter_requires_at_least_one_ip_family(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "connection_filter must enable at least one IP family",
        origin.clone(),
        "Set ip_family.ipv4 and/or ip_family.ipv6 to true.",
    )
}

pub(crate) fn invalid_cidr_in_connection_filter_allow_list(
    cidr: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("invalid CIDR in connection_filter.cidr.allow: {cidr}"),
        origin.clone(),
        "CIDR must be a valid IPv4 or IPv6 network (e.g. 10.0.0.0/8).",
    )
}

pub(crate) fn invalid_cidr_in_connection_filter_deny_list(
    cidr: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("invalid CIDR in connection_filter.cidr.deny: {cidr}"),
        origin.clone(),
        "CIDR must be a valid IPv4 or IPv6 network (e.g. 192.168.0.0/16).",
    )
}
