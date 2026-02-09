use super::super::connection_rate_limiter_filter::ConnectionRateLimiterFilter;
use crate::conf::types::ConnectionRateLimiterFilterConfig;
use pingora::listeners::ConnectionFilter;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::time::sleep;

//-----------------------------------------------------------------------------
// No Peer Address Handling
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_no_peer_addr_is_rejected() {
    // Arrange
    let config = ConnectionRateLimiterFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 10.0,
    };
    let filter = ConnectionRateLimiterFilter::from(config);

    // Act
    let result = filter.should_accept(None).await;

    // Assert
    assert!(!result);
}

//-----------------------------------------------------------------------------
// Basic Allow / Deny Behavior
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_single_connection_under_limit_is_allowed() {
    // Arrange
    let config = ConnectionRateLimiterFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 10.0,
    };
    let filter = ConnectionRateLimiterFilter::from(config);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    assert!(result);
}

#[tokio::test]
async fn test_exceeding_rate_eventually_rejects() {
    // Arrange
    let config = ConnectionRateLimiterFilterConfig {
        reaction_interval: Duration::from_millis(1),
        max_connections_per_second: 1.0,
    };
    let filter = ConnectionRateLimiterFilter::from(config);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // Act
    let mut accepted = 0;
    let mut rejected = 0;

    for _ in 0..20 {
        if filter.should_accept(Some(&addr)).await {
            accepted += 1;
            // Artificially slow down the rate limiter
            sleep(Duration::from_millis(1)).await;
        } else {
            rejected += 1;
        }
    }

    // Assert
    assert!(accepted > 0, "expected some connections to be accepted");
    assert!(rejected > 0, "expected some connections to be rejected");
}

//-----------------------------------------------------------------------------
// Per-IP Isolation
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_rate_is_tracked_per_ip() {
    // Arrange
    let config = ConnectionRateLimiterFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 3.0,
    };
    let filter = ConnectionRateLimiterFilter::from(config);

    let addr_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80);
    let addr_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 80);

    // Saturate IP A
    for _ in 0..20 {
        let _ = filter.should_accept(Some(&addr_a)).await;
    }

    // Act
    let result_b = filter.should_accept(Some(&addr_b)).await;

    // Assert
    assert!(result_b, "rate limiting should be isolated per source IP");
}
