use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// e.g. "0.0.0.0:8080"
    pub listen: String,

    /// Optional number of worker threads.
    pub threads: Option<usize>,

    /// Optional pid file path
    pub pid_file: Option<String>,
}
