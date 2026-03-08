use crate::enrichment::user_agent::{DeviceType, UserAgentInfo};
use uaparser::{Parser, UserAgentParser};

pub struct UaParserEngine {
    parser: UserAgentParser,
}

impl UaParserEngine {
    pub fn new(regexes_yaml: &[u8]) -> anyhow::Result<Self> {
        let parser = UserAgentParser::from_bytes(regexes_yaml)?;
        Ok(Self { parser })
    }

    pub fn parse(&self, ua: &str) -> UserAgentInfo {
        let client = self.parser.parse(ua);

        let ua_family = client.user_agent.family.to_lowercase();
        let device_family = client.device.family.to_lowercase();

        let is_bot = ua_family.contains("bot")
            || ua_family.contains("crawler")
            || ua_family.contains("spider");

        let device_type = if is_bot {
            DeviceType::Bot
        } else if device_family.contains("mobile") || ua.contains("Mobile") {
            DeviceType::Mobile
        } else if device_family.contains("tablet") {
            DeviceType::Tablet
        } else {
            DeviceType::Desktop
        };

        UserAgentInfo {
            device_type,
            is_bot,
        }
    }
}
