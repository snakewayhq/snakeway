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

    pub enable_geoip: bool,

    pub geoip_city_db: Option<PathBuf>,
    pub geoip_isp_db: Option<PathBuf>,
    pub geoip_connection_type_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    pub ua_engine: UaEngineSpec,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineSpec {
    UaParser,
    #[default]
    Woothee,
}
