use crate::net::CidrCollection;
use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use snakeway_conf::types::{NetworkConnectionFilterConfig, OnNoPeerAddr};
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};
use tracing::debug;

#[derive(Debug, Default, Clone)]
pub(crate) struct NetworkConnectionFilter {
    pub(crate) cidr_allow: CidrCollection,
    pub(crate) cidr_deny: CidrCollection,
    pub(crate) ip_family_ipv4: bool,
    pub(crate) ip_family_ipv6: bool,
    pub(crate) on_no_peer_addr: OnNoPeerAddr,
}

#[async_trait]
impl ConnectionFilter for NetworkConnectionFilter {
    async fn should_accept(&self, addr_opt: Option<&SocketAddr>) -> bool {
        // If we do not have a peer address, defer to the configured default behavior.
        let addr = match addr_opt {
            Some(a) => a,
            None => return matches!(self.on_no_peer_addr, OnNoPeerAddr::Allow),
        };

        // Check IP family gating before any allow/deny list checks.
        let client_ip = addr.ip();

        match client_ip {
            IpAddr::V4(_) if !self.ip_family_ipv4 => {
                debug!(%client_ip, "Connection rejected as IPv4 is disabled in filter config");
                return false;
            }
            IpAddr::V6(_) if !self.ip_family_ipv6 => {
                debug!(%client_ip, "Connection rejected as IPv6 is disabled in filter config");
                return false;
            }
            _ => {}
        }

        // Any explicit deny entry takes precedence.
        // If there is at least one deny entry and that client IP is in the deny list, reject.
        if !self.cidr_deny.is_empty() && self.cidr_deny.contains(client_ip) {
            debug!(%client_ip, "Connection rejected by CIDR deny list");
            return false;
        }

        // Finally, if the allow list is empty OR the client IP is in the allow list, accept.
        if self.cidr_allow.is_empty() || self.cidr_allow.contains(client_ip) {
            true
        } else {
            debug!(%client_ip, "Connection rejected by CIDR allow list");
            false
        }
    }
}

impl From<NetworkConnectionFilterConfig> for NetworkConnectionFilter {
    fn from(config: NetworkConnectionFilterConfig) -> Self {
        Self {
            cidr_allow: CidrCollection::new(&config.cidr_allow),
            cidr_deny: CidrCollection::new(&config.cidr_deny),
            ip_family_ipv4: config.ip_family_ipv4,
            ip_family_ipv6: config.ip_family_ipv6,
            on_no_peer_addr: config.on_no_peer_addr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pingora::listeners::ConnectionFilter;
    use snakeway_conf::types::{NetworkConnectionFilterConfig, OnNoPeerAddr};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    //-----------------------------------------------------------------------------
    // No Peer Address Handling Tests
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_no_peer_addr_allow() {
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
    async fn test_no_peer_addr_deny() {
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

    //-----------------------------------------------------------------------------
    // IP Family Gating Tests
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_ipv4_enabled() {
        // Arrange
        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let filter = NetworkConnectionFilter {
            ip_family_ipv4: true,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&ipv4_addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_ipv4_disabled() {
        // Arrange
        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let filter = NetworkConnectionFilter {
            ip_family_ipv4: false,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&ipv4_addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_ipv6_enabled() {
        // Arrange
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);
        let filter = NetworkConnectionFilter {
            ip_family_ipv6: true,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&ipv6_addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_ipv6_disabled() {
        // Arrange
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);
        let filter = NetworkConnectionFilter {
            ip_family_ipv6: false,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&ipv6_addr)).await;

        // Assert
        assert!(!result);
    }

    //-----------------------------------------------------------------------------
    // CIDR Allow List Tests
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_cidr_allow_empty_accepts_all() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::default(),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_cidr_allow_ip_in_list() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)), 8080);
        let cidr = "10.0.0.0/8".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_cidr_allow_ip_not_in_list() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let cidr = "10.0.0.0/8".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    //-----------------------------------------------------------------------------
    // CIDR Deny List Tests
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_cidr_deny_ip_in_list() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)), 8080);
        let cidr = "10.0.0.0/8".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_deny: CidrCollection::new(&[cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_cidr_deny_ip_not_in_list() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let cidr = "10.0.0.0/8".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_deny: CidrCollection::new(&[cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    //-----------------------------------------------------------------------------
    // CIDR Precedence Tests
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_cidr_deny_takes_precedence_over_allow() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let cidr = "192.168.1.0/24".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[cidr]),
            cidr_deny: CidrCollection::new(&[cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_cidr_deny_precedence_with_overlapping_ranges_allow_only() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[allow_cidr]),
            cidr_deny: CidrCollection::new(&[deny_cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_cidr_deny_precedence_with_overlapping_ranges_both() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[allow_cidr]),
            cidr_deny: CidrCollection::new(&[deny_cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_cidr_deny_precedence_with_overlapping_ranges_neither() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let filter = NetworkConnectionFilter {
            cidr_allow: CidrCollection::new(&[allow_cidr]),
            cidr_deny: CidrCollection::new(&[deny_cidr]),
            ip_family_ipv4: true,
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ..Default::default()
        };
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    //-----------------------------------------------------------------------------
    // Integration Tests with ConnectionFilterConfig
    //-----------------------------------------------------------------------------

    #[tokio::test]
    async fn test_from_config_default_allow_all_ipv4() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_default_allow_all_ipv6() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8080);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_ip_family_gating_v4_only_accepts_ipv4() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: false,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_ip_family_gating_v4_only_rejects_ipv6() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: false,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888)),
            80,
        );

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_from_config_ip_family_gating_v6_only_rejects_ipv4() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: false,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_from_config_ip_family_gating_v6_only_accepts_ipv6() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: false,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888)),
            80,
        );

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_cidr_allow_list_accepts_allowed() {
        // Arrange
        let allow_cidr = "10.0.0.0/8".parse().unwrap();
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![allow_cidr],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 254)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_cidr_allow_list_rejects_denied() {
        // Arrange
        let allow_cidr = "10.0.0.0/8".parse().unwrap();
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![allow_cidr],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_from_config_cidr_deny_precedence_accepts_allowed() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![allow_cidr],
            cidr_deny: vec![deny_cidr],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_cidr_deny_precedence_rejects_denied() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![allow_cidr],
            cidr_deny: vec![deny_cidr],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_from_config_cidr_deny_precedence_rejects_other() {
        // Arrange
        let deny_cidr = "192.168.1.0/24".parse().unwrap();
        let allow_cidr = "192.168.0.0/16".parse().unwrap();
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![allow_cidr],
            cidr_deny: vec![deny_cidr],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80);

        // Act
        let result = filter.should_accept(Some(&addr)).await;

        // Assert
        assert!(!result);
    }

    #[tokio::test]
    async fn test_from_config_no_peer_addr_allow() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Allow,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);

        // Act
        let result = filter.should_accept(None).await;

        // Assert
        assert!(result);
    }

    #[tokio::test]
    async fn test_from_config_no_peer_addr_deny() {
        // Arrange
        let config = NetworkConnectionFilterConfig {
            cidr_allow: vec![],
            cidr_deny: vec![],
            on_no_peer_addr: OnNoPeerAddr::Deny,
            ip_family_ipv4: true,
            ip_family_ipv6: true,
        };
        let filter = NetworkConnectionFilter::from(config);

        // Act
        let result = filter.should_accept(None).await;

        // Assert
        assert!(!result);
    }
}
