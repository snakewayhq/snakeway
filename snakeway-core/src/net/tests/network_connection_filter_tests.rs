use super::super::network_connection_filter::NetworkConnectionFilter;
use crate::conf::types::{ConnectionFilterConfig, OnNoPeerAddr};
use crate::net::CidrCollection;
use pingora::listeners::ConnectionFilter;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[tokio::test]
async fn test_should_accept_no_peer_addr_allow() {
    // Arrange
    let filter = NetworkConnectionFilter {
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(None).await;

    // Assert
    assert!(result);
}

#[tokio::test]
async fn test_should_accept_no_peer_addr_deny() {
    // Arrange
    let filter = NetworkConnectionFilter {
        on_no_peer_addr: OnNoPeerAddr::Deny,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(None).await;

    // Assert
    assert!(!result);
}

#[tokio::test]
async fn test_should_accept_ipv4_gating() {
    // Arrange
    let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let filter_v4_enabled = NetworkConnectionFilter {
        ip_family_ipv4: true,
        ..Default::default()
    };
    let filter_v4_disabled = NetworkConnectionFilter {
        ip_family_ipv4: false,
        ..Default::default()
    };

    // Act
    let result_enabled = filter_v4_enabled.should_accept(Some(&ipv4_addr)).await;
    let result_disabled = filter_v4_disabled.should_accept(Some(&ipv4_addr)).await;

    // Assert
    assert!(result_enabled);
    assert!(!result_disabled);
}

#[tokio::test]
async fn test_should_accept_ipv6_gating() {
    // Arrange
    let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);

    let filter_v6_enabled = NetworkConnectionFilter {
        ip_family_ipv6: true,
        ..Default::default()
    };
    let filter_v6_disabled = NetworkConnectionFilter {
        ip_family_ipv6: false,
        ..Default::default()
    };

    // Act
    let result_enabled = filter_v6_enabled.should_accept(Some(&ipv6_addr)).await;
    let result_disabled = filter_v6_disabled.should_accept(Some(&ipv6_addr)).await;

    // Assert
    assert!(result_enabled);
    assert!(!result_disabled);
}

#[tokio::test]
async fn test_should_accept_cidr_deny_precedence() {
    // Arrange
    let ip = Ipv4Addr::new(192, 168, 1, 1);
    let addr = SocketAddr::new(IpAddr::V4(ip), 8080);
    let cidr = "192.168.1.0/24".parse().unwrap();

    let filter = NetworkConnectionFilter {
        cidr_allow: CidrCollection::new(&[cidr]),
        cidr_deny: CidrCollection::new(&[cidr]),
        ip_family_ipv4: true,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    // Deny list takes precedence over allow list
    assert!(!result);
}

#[tokio::test]
async fn test_should_accept_cidr_allow_empty() {
    // Arrange
    let ip = Ipv4Addr::new(192, 168, 1, 1);
    let addr = SocketAddr::new(IpAddr::V4(ip), 8080);

    let filter = NetworkConnectionFilter {
        cidr_allow: CidrCollection::default(),
        ip_family_ipv4: true,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    // Empty allow list means everything is allowed (if not denied)
    assert!(result);
}

#[tokio::test]
async fn test_should_accept_cidr_allow_not_in_list() {
    // Arrange
    let ip = Ipv4Addr::new(192, 168, 1, 1);
    let addr = SocketAddr::new(IpAddr::V4(ip), 8080);
    let cidr = "10.0.0.0/8".parse().unwrap();

    let filter = NetworkConnectionFilter {
        cidr_allow: CidrCollection::new(&[cidr]),
        ip_family_ipv4: true,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    // Not in allow list when allow list is not empty
    assert!(!result);
}

#[tokio::test]
async fn test_should_accept_cidr_allow_in_list() {
    // Arrange
    let ip = Ipv4Addr::new(10, 0, 0, 5);
    let addr = SocketAddr::new(IpAddr::V4(ip), 8080);
    let cidr = "10.0.0.0/8".parse().unwrap();

    let filter = NetworkConnectionFilter {
        cidr_allow: CidrCollection::new(&[cidr]),
        ip_family_ipv4: true,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    assert!(result);
}

#[tokio::test]
async fn test_should_accept_cidr_deny_list() {
    // Arrange
    let ip = Ipv4Addr::new(10, 0, 0, 5);
    let addr = SocketAddr::new(IpAddr::V4(ip), 8080);
    let cidr = "10.0.0.0/8".parse().unwrap();

    let filter = NetworkConnectionFilter {
        cidr_deny: CidrCollection::new(&[cidr]),
        ip_family_ipv4: true,
        ..Default::default()
    };

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    assert!(!result);
}

#[tokio::test]
async fn test_network_connection_filter_default_allow_all() {
    let config = ConnectionFilterConfig {
        cidr_allow: vec![],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: true,
        ip_family_ipv6: true,
    };
    let filter = NetworkConnectionFilter::from(config);

    let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);

    assert!(filter.should_accept(Some(&ipv4_addr)).await);
    assert!(filter.should_accept(Some(&ipv6_addr)).await);
}

#[tokio::test]
async fn test_network_connection_filter_ip_family_gating() {
    // Only IPv4 allowed
    let config_v4 = ConnectionFilterConfig {
        cidr_allow: vec![],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: true,
        ip_family_ipv6: false,
    };
    let filter_v4 = NetworkConnectionFilter::from(config_v4);

    let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 80);
    let ipv6_addr = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888)),
        80,
    );

    assert!(filter_v4.should_accept(Some(&ipv4_addr)).await);
    assert!(!filter_v4.should_accept(Some(&ipv6_addr)).await);

    // Only IPv6 allowed
    let config_v6 = ConnectionFilterConfig {
        cidr_allow: vec![],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: false,
        ip_family_ipv6: true,
    };
    let filter_v6 = NetworkConnectionFilter::from(config_v6);

    assert!(!filter_v6.should_accept(Some(&ipv4_addr)).await);
    assert!(filter_v6.should_accept(Some(&ipv6_addr)).await);
}

#[tokio::test]
async fn test_network_connection_filter_cidr_deny_precedence() {
    let deny_cidr = "192.168.1.0/24".parse().unwrap();
    let allow_cidr = "192.168.0.0/16".parse().unwrap();

    let config = ConnectionFilterConfig {
        cidr_allow: vec![allow_cidr],
        cidr_deny: vec![deny_cidr],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: true,
        ip_family_ipv6: true,
    };
    let filter = NetworkConnectionFilter::from(config);

    // In allow list, not in deny list
    let allowed_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 80);
    // In both allow and deny list
    let denied_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 80);
    // In neither
    let other_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80);

    assert!(filter.should_accept(Some(&allowed_addr)).await);
    assert!(!filter.should_accept(Some(&denied_addr)).await);
    assert!(!filter.should_accept(Some(&other_addr)).await);
}

#[tokio::test]
async fn test_network_connection_filter_cidr_allow_list() {
    let allow_cidr = "10.0.0.0/8".parse().unwrap();

    let config = ConnectionFilterConfig {
        cidr_allow: vec![allow_cidr],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: true,
        ip_family_ipv6: true,
    };
    let filter = NetworkConnectionFilter::from(config);

    let allowed_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 254)), 80);
    let denied_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 1)), 80);

    assert!(filter.should_accept(Some(&allowed_addr)).await);
    assert!(!filter.should_accept(Some(&denied_addr)).await);
}

#[tokio::test]
async fn test_network_connection_filter_on_no_peer_addr() {
    // Allow when no peer addr
    let config_allow = ConnectionFilterConfig {
        cidr_allow: vec![],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Allow,
        ip_family_ipv4: true,
        ip_family_ipv6: true,
    };
    let filter_allow = NetworkConnectionFilter::from(config_allow);
    assert!(filter_allow.should_accept(None).await);

    // Deny when no peer addr
    let config_deny = ConnectionFilterConfig {
        cidr_allow: vec![],
        cidr_deny: vec![],
        on_no_peer_addr: OnNoPeerAddr::Deny,
        ip_family_ipv4: true,
        ip_family_ipv6: true,
    };
    let filter_deny = NetworkConnectionFilter::from(config_deny);
    assert!(!filter_deny.should_accept(None).await);
}
