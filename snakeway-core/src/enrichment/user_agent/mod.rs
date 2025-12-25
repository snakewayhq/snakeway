mod uaparser_engine;
mod woothee_engine;

use crate::conf::types::UaEngineKind;
use crate::enrichment::user_agent::uaparser_engine::UaParserEngine;
use crate::enrichment::user_agent::woothee_engine::WootheeEngine;
use std::net::IpAddr;

const REGEXES_YAML: &[u8] = include_bytes!("regexes.yaml");

pub fn build_ua_engine(kind: UaEngineKind) -> anyhow::Result<UaEngine> {
    match kind {
        UaEngineKind::UaParser => Ok(UaEngine::UaParser(UaParserEngine::new(REGEXES_YAML)?)),
        UaEngineKind::Woothee => Ok(UaEngine::Woothee(WootheeEngine::new())),
    }
}

pub enum UaEngine {
    UaParser(UaParserEngine),
    Woothee(WootheeEngine),
}

impl UaEngine {
    pub fn parse(&self, ua: &str) -> UserAgentInfo {
        match self {
            UaEngine::UaParser(p) => p.parse(ua),
            UaEngine::Woothee(p) => p.parse(ua),
        }
    }
}

/// Dead fields aren't really dead - they just might not be used by built-in devices.
#[derive(Debug, Clone)]
pub struct ClientIdentity {
    #[allow(dead_code)]
    pub ip: IpAddr,
    #[allow(dead_code)]
    /// empty unless trusted proxies enabled/used
    pub proxy_chain: Vec<IpAddr>,
    pub geo: Option<GeoInfo>,
    pub ua: Option<UserAgentInfo>,
}

#[derive(Debug, Clone)]
pub struct GeoInfo {
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub asn: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UserAgentInfo {
    pub device_type: DeviceType,
    pub is_bot: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Bot,
    Unknown,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Desktop => "desktop",
            DeviceType::Mobile => "mobile",
            DeviceType::Tablet => "tablet",
            DeviceType::Bot => "bot",
            DeviceType::Unknown => "unknown",
        }
    }
}
