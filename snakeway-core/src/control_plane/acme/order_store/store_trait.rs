use serde::{Deserialize, Serialize};
use snakeway_conf::types::AcmeChallengeConfig;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OrderState {
    pub(crate) cert_id: String,
    pub(crate) domains: Vec<String>,
    pub(crate) challenge: AcmeChallengeConfig,

    pub(crate) status: OrderStatus,

    /// ACME order URL
    pub(crate) order_url: String,

    /// Authorization URLs returned by ACME
    pub(crate) authorization_urls: Vec<String>,

    /// HTTP-01 challenge tokens: each entry is a (token, keyAuthorization) pair,
    /// one per pending ACME authorization. A single-domain order has one entry;
    /// a SAN order covering N domains has N entries.
    pub(crate) challenge_tokens: Vec<(String, String)>,

    /// Failure count for backoff
    pub(crate) failure_count: u32,

    /// Last error string
    pub(crate) last_error: Option<String>,

    /// Last update timestamp
    pub(crate) updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum OrderStatus {
    Ordering,
    ChallengeInit,
    Challenging,
    Finalizing,
    Failed,
}

pub trait OrderStore: Send + Sync {
    fn get(&self, cert_id: &str) -> std::io::Result<Option<OrderState>>;
    fn put(&self, state: &OrderState) -> std::io::Result<()>;
    fn delete(&self, cert_id: &str) -> std::io::Result<()>;
    fn list(&self) -> std::io::Result<Vec<OrderState>>;
}
