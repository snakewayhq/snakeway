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
    fn from(spec: IdentityDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            trusted_proxies: spec.trusted_proxies,
            enable_geoip: spec.enable_geoip,
            geoip_db: spec.geoip_db,
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
