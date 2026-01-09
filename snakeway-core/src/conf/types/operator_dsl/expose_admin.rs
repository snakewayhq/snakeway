use crate::conf::types::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindAdminConfig {
    pub addr: String,
    pub tls: TlsConfig,
}
