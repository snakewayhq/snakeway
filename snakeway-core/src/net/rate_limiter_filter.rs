use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use pingora_limits::rate::Rate;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ConnectionRateLimiter {
    /// Rate estimator (per key)
    rate: Arc<Rate>,
    /// Maximum allowed connections per second per IP
    max_connections_per_second: f64,
}

impl ConnectionRateLimiter {
    /// Create a new connection rate limiter.
    ///
    /// `interval` controls how quickly the estimator reacts.
    /// `max_connections_per_second` is the enforcement threshold.
    pub fn new(interval: Duration, max_connections_per_second: f64) -> Self {
        Self {
            rate: Arc::new(Rate::new(interval)),
            max_connections_per_second,
        }
    }
}

impl Default for ConnectionRateLimiter {
    fn default() -> Self {
        // Sensible, conservative defaults:
        // 1 second window
        // 50 new connections/sec per IP
        Self::new(Duration::from_secs(1), 50.0)
    }
}

impl Debug for ConnectionRateLimiter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionRateLimiter")
            .field("max_connections_per_sec", &self.max_connections_per_second)
            .finish()
    }
}
#[async_trait]
impl ConnectionFilter for ConnectionRateLimiter {
    async fn should_accept(&self, addr_opt: Option<&SocketAddr>) -> bool {
        let addr = match addr_opt {
            Some(addr) => addr,
            None => {
                // No peer address, then fail
                return false;
            }
        };

        let ip = addr.ip();

        // Observe this connection attempt.
        self.rate.observe(&ip, 1);

        // Get the estimated connection rate (connections/sec).
        let current_rate = self.rate.rate(&ip);

        // Enforce policy.
        current_rate <= self.max_connections_per_second
    }
}

// impl From<ConnectionRateLimiterConfig> for ConnectionRateLimiter {
//     fn from(config: ConnectionFilterConfig) -> Self {
//         Self {}
//     }
// }
