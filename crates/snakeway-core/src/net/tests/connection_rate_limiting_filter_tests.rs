use super::super::connection_rate_limiting_filter::ConnectionRateLimitingFilter;
use pingora::listeners::ConnectionFilter;
use snakeway_conf::types::ConnectionRateLimitingFilterConfig;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::time::sleep;

//-----------------------------------------------------------------------------
// No Peer Address Handling
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_no_peer_addr_is_rejected() {
    // Arrange
    let config = ConnectionRateLimitingFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 10.0,
    };
    let filter = ConnectionRateLimitingFilter::from(config);

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
    let config = ConnectionRateLimitingFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 10.0,
    };
    let filter = ConnectionRateLimitingFilter::from(config);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // Act
    let result = filter.should_accept(Some(&addr)).await;

    // Assert
    assert!(result);
}

#[tokio::test]
async fn test_exceeding_rate_eventually_rejects_across_intervals() {
    // Arrange
    let reaction_interval = Duration::from_millis(200);
    let config = ConnectionRateLimitingFilterConfig {
        reaction_interval,
        max_connections_per_second: 3.0,
    };
    let filter = ConnectionRateLimitingFilter::from(config);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // Act (apply pressure across multiple intervals)
    let start = std::time::Instant::now();
    let mut saw_reject = false;
    while start.elapsed() < reaction_interval * 3 {
        if !filter.should_accept(Some(&addr)).await {
            saw_reject = true;
            break;
        }
        // Yield to allow time to advance and allow an interval rollover to happen.
        tokio::task::yield_now().await;
    }

    // Assert
    assert!(
        saw_reject,
        "expected rejection once pressure spans multiple intervals"
    );
}

//-----------------------------------------------------------------------------
// Per-IP Isolation
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_rate_is_tracked_per_ip() {
    // Arrange
    let config = ConnectionRateLimitingFilterConfig {
        reaction_interval: Duration::from_secs(1),
        max_connections_per_second: 3.0,
    };
    let filter = ConnectionRateLimitingFilter::from(config);

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

//-----------------------------------------------------------------------------
// Rate Decay / Recovery
//-----------------------------------------------------------------------------

#[tokio::test]
async fn test_rate_does_not_permanently_reject_after_pressure() {
    // Arrange
    let reaction_interval = Duration::from_millis(100);
    let config = ConnectionRateLimitingFilterConfig {
        reaction_interval,
        max_connections_per_second: 2.0,
    };
    let filter = ConnectionRateLimitingFilter::from(config);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    // Act (apply pressure)
    // Apply pressure across multiple intervals.
    let start = std::time::Instant::now();
    while start.elapsed() < reaction_interval * 3 {
        let _ = filter.should_accept(Some(&addr)).await;
        tokio::task::yield_now().await;
    }
    // Stop traffic completely.
    sleep(reaction_interval * 2).await;

    // Try multiple times to see if we ever get an allow/
    let mut saw_allow = false;
    for _ in 0..50 {
        if filter.should_accept(Some(&addr)).await {
            saw_allow = true;
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }

    assert!(
        saw_allow,
        "expected limiter to eventually allow after traffic stops"
    );
}
