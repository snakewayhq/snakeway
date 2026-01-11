use crate::conf::types::{Origin, TlsConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub enable_http2: bool,
}
