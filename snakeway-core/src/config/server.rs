use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// e.g. "0.0.0.0:8080"
    pub listen: String,

    /// Optional pid file path
    pub pid_file: Option<String>,
}
