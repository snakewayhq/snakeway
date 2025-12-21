use http::HeaderMap;
use ipnet::IpNet;
use std::net::IpAddr;

/// Resolve the true client IP using X-Forwarded-For and a trusted proxy list.
///
/// Returns:
/// - client_ip: the resolved client IP
/// - proxy_chain: ordered list of proxy IPs (closest first)
///
/// Rules:
/// - Walk XFF from right → left
/// - Stop at first IP not in trusted_proxies
/// - If no untrusted IP found, fall back to peer_ip
pub fn resolve_client_ip(
    headers: &HeaderMap,
    peer_ip: IpAddr,
    trusted_proxies: &[IpNet],
) -> (IpAddr, Vec<IpAddr>) {
    // Fast path: no XFF or no trusted proxies
    let xff = match headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return (peer_ip, Vec::new()),
    };

    if trusted_proxies.is_empty() {
        return (peer_ip, Vec::new());
    }

    let mut proxy_chain = Vec::new();

    // Parse XFF into IPs (left = client, right = closest proxy)
    let ips: Vec<IpAddr> = xff
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<IpAddr>().ok())
        .collect();

    // Walk from closest → farthest
    for ip in ips.iter().rev() {
        if trusted_proxies.iter().any(|net| net.contains(ip)) {
            proxy_chain.push(*ip);
            continue;
        }

        // First untrusted IP is the client
        return (*ip, proxy_chain);
    }

    // All XFF entries were trusted proxies → fall back
    (peer_ip, proxy_chain)
}
