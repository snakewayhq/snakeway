use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// Load balancing strategy
    #[serde(default = "default_strategy")]
    pub strategy: Strategy,

    #[serde(default)]
    pub upstream: Vec<UpstreamConfig>,
}

fn default_strategy() -> Strategy {
    Strategy::Failover
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Failover,
    RoundRobin,
    LeastConnections,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub url: String,

    /// Optional weight
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
}
