use crate::types::HclOrigin;
use confval::ValidationIssue;
use std::net::IpAddr;
use std::path::Path;

pub(crate) fn service_has_no_upstreams(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("service has no upstream backends", origin.clone())
}

pub(crate) fn invalid_upstream_weight(
    weight: &i64,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid upstream weight: {}", weight),
        origin.clone(),
    )
}

pub(crate) fn upstream_cannot_have_both_sock_and_endpoint(
    sock: &str,
    host: &str,
    port: i64,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "upstream cannot have both sock {} and endpoint: {}:{}",
            sock, host, port
        ),
        origin.clone(),
    )
}

pub(crate) fn upstream_must_have_a_sock_or_endpoint(
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        "invalid upstream - it must have a sock or an endpoint, but neither are defined",
        origin.clone(),
        "Only one can be set.",
    )
}

pub(crate) fn duplicate_upstream_sock(
    sock: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("duplicate upstream sock: {}", sock), origin.clone())
}

pub(crate) fn route_has_no_hosts(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("route has no hosts", origin.clone())
}

pub(crate) fn upstream_tls_sni_required(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("upstream TLS SNI required", origin.clone())
}

pub(crate) fn upstream_tls_sni_must_be_dns(origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error("upstream TLS SNI must be DNS name", origin.clone())
}

pub(crate) fn upstream_tls_has_invalid_ca_file(
    ca_file: &Path,
    err: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!(
            "upstream TLS has invalid CA file ({}): {}",
            ca_file.to_string_lossy(),
            err
        ),
        origin.clone(),
    )
}

pub(crate) fn duplicate_route_path(path: &str, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error_with_help(
        format!("duplicate route path within the same listener: {path}"),
        origin.clone(),
        "Each route path must be unique per listener. Use different path prefixes or move the route to a separate ingress file.",
    )
}

pub(crate) fn websocket_route_cannot_be_used_with_http2(
    path: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("websocket route cannot be used with HTTP2: {}", path),
        origin.clone(),
    )
}

pub(crate) fn invalid_upstream_ip(ip: &IpAddr, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid upstream ip: {}", ip), origin.clone())
}

pub(crate) fn invalid_upstream_hostname(
    hostname: &str,
    origin: &HclOrigin,
) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(
        format!("invalid upstream hostname: {}", hostname),
        origin.clone(),
    )
}
