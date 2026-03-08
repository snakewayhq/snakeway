use crate::conf::types::{IdentityDeviceSpec, UaEngineSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityDeviceConfig {
    pub(crate) enable: bool,

    /// CIDR strings
    pub(crate) trusted_proxies: Vec<String>,

    pub(crate) max_x_forwarded_for_length: usize,

    pub(crate) enable_geoip: bool,

    pub(crate) geoip_city_db: Option<PathBuf>,
    pub(crate) geoip_isp_db: Option<PathBuf>,
    pub(crate) geoip_connection_type_db: Option<PathBuf>,

    pub(crate) enable_user_agent: bool,

    pub(crate) ua_engine: UaEngineKind,

    pub(crate) max_user_agent_length: usize,
}

impl From<IdentityDeviceSpec> for IdentityDeviceConfig {
    fn from(spec: IdentityDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            trusted_proxies: spec.trusted_proxies,
            max_x_forwarded_for_length: spec.max_x_forwarded_for_length,
            enable_geoip: spec.enable_geoip,
            geoip_city_db: spec.geoip_city_db,
            geoip_isp_db: spec.geoip_isp_db,
            geoip_connection_type_db: spec.geoip_connection_type_db,
            enable_user_agent: spec.enable_user_agent,
            ua_engine: spec.ua_engine.into(),
            max_user_agent_length: spec.max_user_agent_length,
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UaEngineKind {
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
