use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Address to bind, e.g. "0.0.0.0:8080"
    pub listen: String,

    /// Optional number of worker threads.
    pub threads: Option<usize>,

    /// Optional pid file path
    pub pid_file: Option<String>,

    /// Optional TLS config
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}
