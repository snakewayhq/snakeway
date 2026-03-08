use crate::conf::types::ConnectionRateLimitingFilterConfig;
use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use pingora_limits::rate::Rate;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ConnectionRateLimitingFilter {
    /// Rate estimator (per key)
    rate: Arc<Rate>,
    /// Maximum allowed connections per second per IP
    max_connections_per_second: f64,
}

impl Debug for ConnectionRateLimitingFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionRateLimiter")
            .field("max_connections_per_sec", &self.max_connections_per_second)
            .finish()
    }
}
#[async_trait]
impl ConnectionFilter for ConnectionRateLimitingFilter {
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

impl From<ConnectionRateLimitingFilterConfig> for ConnectionRateLimitingFilter {
    fn from(config: ConnectionRateLimitingFilterConfig) -> Self {
        Self {
            rate: Arc::new(Rate::new(config.reaction_interval)),
            max_connections_per_second: config.max_connections_per_second,
        }
    }
}
