use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// e.g. "0.0.0.0:8080"
    pub listen: String,
}
