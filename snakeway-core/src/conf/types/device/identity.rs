use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    /// CIDR strings
    #[serde(default)]
    pub trusted_proxies: Vec<String>,

    #[serde(default)]
    pub enable_geoip: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geoip_db: Option<String>,

    #[serde(default)]
    pub enable_user_agent: bool,

    #[serde(default = "default_ua_engine")]
    pub ua_engine: UaEngineKind,
}

fn default_ua_engine() -> UaEngineKind {
    UaEngineKind::Woothee
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    Woothee,
}
