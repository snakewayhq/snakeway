use crate::conf::types::{Origin, TlsSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct BindSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: String,
    pub tls: Option<TlsSpec>,
    pub enable_http2: bool,
    pub redirect_http_to_https: Option<RedirectSpec>,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct RedirectSpec {
    pub port: u16,
    pub status: u16,
}
