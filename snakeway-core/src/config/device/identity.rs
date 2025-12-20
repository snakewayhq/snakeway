use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    #[serde(default)]
    pub trusted_proxies: Vec<String>, // CIDR strings (future use)

    #[serde(default)]
    pub enable_geoip: bool,

    #[serde(default)]
    pub geoip_db: Option<String>,

    #[serde(default)]
    pub enable_user_agent: bool,

    #[serde(default = "default_ua_engine")]
    pub ua_engine: UaEngineKind,
}

fn default_ua_engine() -> UaEngineKind {
    UaEngineKind::Woothee
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: vec![],
            enable_geoip: true,
            geoip_db: None,
            enable_user_agent: true,
            ua_engine: UaEngineKind::Woothee,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    Woothee,
}
