mod uaparser_engine;
mod woothee_engine;

use crate::execution::enrichment::user_agent::uaparser_engine::UaParserEngine;
use crate::execution::enrichment::user_agent::woothee_engine::WootheeEngine;
use snakeway_conf::types::UaEngineKind;
use std::net::IpAddr;
use std::path::Path;

const REGEXES_YAML: &[u8] = include_bytes!("regexes.yaml");

pub(crate) fn build_ua_engine(
    kind: UaEngineKind,
    ua_parser_regexes: Option<&Path>,
) -> anyhow::Result<UaEngine> {
    match kind {
        UaEngineKind::UaParser => {
            let regexes = match ua_parser_regexes {
                Some(path) => std::fs::read(path)?,
                None => REGEXES_YAML.to_vec(),
            };
            Ok(UaEngine::UaParser(UaParserEngine::new(&regexes)?))
        }
        UaEngineKind::Woothee => Ok(UaEngine::Woothee(WootheeEngine::new())),
    }
}

pub(crate) enum UaEngine {
    UaParser(UaParserEngine),
    Woothee(WootheeEngine),
}

impl UaEngine {
    pub(crate) fn parse(&self, ua: &str) -> UserAgentInfo {
        match self {
            UaEngine::UaParser(p) => p.parse(ua),
            UaEngine::Woothee(p) => p.parse(ua),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClientIdentity {
    #[allow(dead_code)]
    pub(crate) ip: IpAddr,
    /// empty unless trusted proxies enabled/used
    pub(crate) proxy_chain: Vec<IpAddr>,
    pub(crate) is_forwarded: bool,
    pub(crate) is_trusted: bool,
    pub(crate) geo: Option<GeoInfo>,
    pub(crate) ua: Option<UserAgentInfo>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GeoInfo {
    /// e.g., US, GB, etc
    pub(crate) country_code: Option<String>,
    /// Location region
    pub(crate) region: Option<String>,
    /// Autonomous System Number
    pub(crate) asn: Option<u32>,
    /// Autonomous System Organization
    pub(crate) aso: Option<String>,
    /// e.g., wifi, mobile, etc
    pub(crate) connection_type: Option<String>,
}

impl GeoInfo {
    pub(crate) fn has_some_info(&self) -> bool {
        self.country_code.is_some()
            || self.region.is_some()
            || self.asn.is_some()
            || self.aso.is_some()
            || self.connection_type.is_some()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UserAgentInfo {
    pub(crate) device_type: DeviceType,
    pub(crate) is_bot: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Bot,
    Unknown,
}

impl DeviceType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Desktop => "desktop",
            DeviceType::Mobile => "mobile",
            DeviceType::Tablet => "tablet",
            DeviceType::Bot => "bot",
            DeviceType::Unknown => "unknown",
        }
    }
}
