use crate::conf::types::{Origin, TlsSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: String,
    pub tls: Option<TlsSpec>,
    pub enable_http2: bool,
}
