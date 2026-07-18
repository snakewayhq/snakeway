use crate::types::UpstreamSourceAddressesConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = UpstreamSettingsSpec)]
pub struct UpstreamSettingsConfig {
    /// Idle upstream keepalive connections kept per worker thread.
    #[confval(lower(from = connection_pool_size, with = narrow::opt_i64_to_usize))]
    pub connection_pool_size: Option<usize>,
    /// Connect timeout (TCP plus TLS). `None` disables it.
    #[confval(lower(from = connection_timeout_seconds, with = narrow::opt_i64_secs_to_duration))]
    pub connection_timeout: Option<Duration>,
    /// Per-read (idle) timeout. `None` disables it.
    #[confval(lower(from = read_timeout_seconds, with = narrow::opt_i64_secs_to_duration))]
    pub read_timeout: Option<Duration>,
    /// Local source addresses for outbound upstream connections.
    #[confval(nested)]
    pub source_addresses: Option<UpstreamSourceAddressesConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = UpstreamSourceAddressesSpec)]
pub struct UpstreamSourceAddressesConfig {
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}
