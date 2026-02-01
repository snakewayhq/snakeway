use ipnet::IpNet;
use std::net::IpAddr;

fn is_addr_in_networks(addr: IpAddr, nets: &[IpNet]) -> bool {
    nets.iter().any(|net| net.contains(&addr))
}

pub fn is_addr_allowed(addr: IpAddr, cidr_allow: &[IpNet]) -> bool {
    cidr_allow.is_empty() || is_addr_in_networks(addr, cidr_allow)
}

pub fn is_addr_denied(addr: IpAddr, cidr_deny: &[IpNet]) -> bool {
    !cidr_deny.is_empty() && is_addr_in_networks(addr, cidr_deny)
}
