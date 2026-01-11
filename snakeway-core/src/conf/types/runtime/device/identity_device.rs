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

    pub geoip_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    pub ua_engine: UaEngineKind,
}

impl From<IdentityDeviceSpec> for IdentityDeviceConfig {
    fn from(d: IdentityDeviceSpec) -> Self {
        Self {
            enable: d.enable,
            trusted_proxies: d.trusted_proxies,
            enable_geoip: d.enable_geoip,
            geoip_db: d.geoip_db,
            enable_user_agent: d.enable_user_agent,
            ua_engine: d.ua_engine.into(),
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
