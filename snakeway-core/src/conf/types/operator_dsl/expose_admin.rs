use crate::conf::types::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ExposeAdminConfig {
    pub addr: String,
    pub tls: TlsConfig,
    pub enable_admin: bool,
}
