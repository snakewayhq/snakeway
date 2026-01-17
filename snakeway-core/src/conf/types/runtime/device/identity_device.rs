use crate::conf::types::{IdentityDeviceSpec, UaEngineSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceConfig {
    pub enable: bool,

    /// CIDR strings
    pub trusted_proxies: Vec<String>,

    pub enable_geoip: bool,

    pub geoip_city_db: Option<PathBuf>,
    pub geoip_isp_db: Option<PathBuf>,
    pub geoip_connection_type_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    pub ua_engine: UaEngineKind,
}

impl From<IdentityDeviceSpec> for IdentityDeviceConfig {
    fn from(spec: IdentityDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            trusted_proxies: spec.trusted_proxies,
            enable_geoip: spec.enable_geoip,
            geoip_city_db: spec.geoip_city_db,
            geoip_isp_db: spec.geoip_isp_db,
            geoip_connection_type_db: spec.geoip_connection_type_db,
            enable_user_agent: spec.enable_user_agent,
            ua_engine: spec.ua_engine.into(),
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    #[default]
    Woothee,
}

impl From<UaEngineSpec> for UaEngineKind {
    fn from(ua_engine: UaEngineSpec) -> Self {
        match ua_engine {
            UaEngineSpec::UaParser => UaEngineKind::UaParser,
            UaEngineSpec::Woothee => UaEngineKind::Woothee,
        }
    }
}
