use crate::types::HclOrigin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ua_parser_regexes: Option<PathBuf>,
    #[serde(default = "default_max_user_agent_length")]
    pub max_user_agent_length: usize,
}

fn default_max_x_forwarded_for_length() -> usize {
    1024
}

fn default_max_user_agent_length() -> usize {
    2048
}

impl Default for IdentityDeviceSpec {
    fn default() -> Self {
        Self {
            origin: Default::default(),
            enable: false,
            trusted_proxies: vec![],
            max_x_forwarded_for_length: default_max_x_forwarded_for_length(),
            enable_geoip: false,
            geoip_city_db: None,
            geoip_isp_db: None,
            geoip_connection_type_db: None,
            enable_user_agent: false,
            ua_engine: Default::default(),
            ua_parser_regexes: None,
            max_user_agent_length: default_max_user_agent_length(),
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineSpec {
    UaParser,
    #[default]
    Woothee,
}
