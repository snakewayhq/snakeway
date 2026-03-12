use http::HeaderMap;
use ipnet::IpNet;
use std::net::IpAddr;

/// Resolve the true client IP using X-Forwarded-For and a trusted proxy list.
///
/// Returns:
/// - client_ip: the resolved client IP
/// - proxy_chain: ordered list of proxy IPs (closest first)
/// - is_forwarded: true if an X-Forwarded-For header was present on the request
/// - is_trusted: true if the forwarded client identity is trusted
///
/// Rules:
/// - Walk XFF from right → left
/// - Stop at first IP not in trusted_proxies
/// - If no untrusted IP found, fall back to peer_ip
pub(crate) fn resolve_client_ip(
    headers: &HeaderMap,
    peer_ip: IpAddr,
    trusted_proxies: &[IpNet],
    max_xff_length: usize,
) -> (IpAddr, Vec<IpAddr>, bool, bool) {
    let xff_header = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok());
    let is_forwarded = xff_header.is_some();

    // If there are no trusted proxies, we can't trust XFF, so just return the peer IP.
    if trusted_proxies.is_empty() {
        return (peer_ip, Vec::new(), is_forwarded, false);
    }

    // Only trust XFF if the immediate peer is trusted
    if !trusted_proxies.iter().any(|net| net.contains(&peer_ip)) {
        return (peer_ip, Vec::new(), is_forwarded, false);
    }

    let xff = match headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return (peer_ip, Vec::new(), is_forwarded, false),
    };

    // Guard against overly long XFF headers to prevent potential abuse.
    if xff.len() > max_xff_length {
        return (peer_ip, Vec::new(), is_forwarded, false);
    }

    let ips: Vec<IpAddr> = xff
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<IpAddr>().ok())
        .collect();

    let mut proxy_chain = Vec::with_capacity(ips.len());

    for ip in ips.iter().rev() {
        if trusted_proxies.iter().any(|net| net.contains(ip)) {
            proxy_chain.push(*ip);
            continue;
        }

        return (*ip, proxy_chain, true, true);
    }

    (peer_ip, proxy_chain, true, true)
}
