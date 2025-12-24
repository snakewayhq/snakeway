use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub name: String,

    /// Load balancing strategy (Phase 2A = "failover")
    pub strategy: Strategy,

    #[serde(default)]
    pub upstream: Vec<UpstreamConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Failover,
    // Phase 2B+
    RoundRobin,
    LeastConnections,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub url: String,

    /// Optional weight (Phase 2B)
    pub weight: Option<u32>,
}
