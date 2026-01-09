use crate::conf::types::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindConfig {
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub enable_http2: bool,
}
