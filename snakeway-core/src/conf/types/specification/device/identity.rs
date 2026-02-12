use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

    pub enable: bool,

    /// CIDR strings
    pub trusted_proxies: Vec<String>,
    #[serde(default = "default_max_x_forwarded_for_length")]
    pub max_x_forwarded_for_length: usize,

    pub enable_geoip: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_city_db: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_isp_db: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_connection_type_db: Option<PathBuf>,

    pub enable_user_agent: bool,
    pub ua_engine: UaEngineSpec,
    #[serde(default = "default_max_user_agent_length")]
    pub max_user_agent_length: usize,
}

fn default_max_x_forwarded_for_length() -> usize {
    1024
}

fn default_max_user_agent_length() -> usize {
    2048
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineSpec {
    UaParser,
    #[default]
    Woothee,
}
