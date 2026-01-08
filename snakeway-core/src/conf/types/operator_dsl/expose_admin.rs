use crate::conf::types::TlsConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ExposeAdminConfig {
    pub addr: String,
    pub tls: TlsConfig,
    pub enable_admin: bool,
}
