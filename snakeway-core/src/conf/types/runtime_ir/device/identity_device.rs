use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceConfig {
    pub enable: bool,

    /// CIDR strings
    pub trusted_proxies: Vec<String>,

    pub enable_geoip: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geoip_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    pub ua_engine: UaEngineKind,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    #[default]
    Woothee,
}
