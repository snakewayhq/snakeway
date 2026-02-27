use crate::conf::types::AcmeChallengeConfig;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderState {
    pub cert_id: String,
    pub domains: Vec<String>,
    pub challenge: AcmeChallengeConfig,

    pub status: OrderStatus,

    /// ACME order URL
    pub order_url: String,

    /// Authorization URLs returned by ACME
    pub authorization_urls: Vec<String>,

    /// Selected challenge auth URL (HTTP-01)
    pub challenge_url: Option<String>,

    /// HTTP-01 token
    pub challenge_token: Option<String>,

    /// keyAuthorization
    pub challenge_key_authorization: Option<String>,

    /// Failure count for backoff
    pub failure_count: u32,

    /// Last error string
    pub last_error: Option<String>,

    /// Last update timestamp
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
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
