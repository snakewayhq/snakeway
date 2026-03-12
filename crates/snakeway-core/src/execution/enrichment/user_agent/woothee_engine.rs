use crate::execution::enrichment::user_agent::{DeviceType, UserAgentInfo};
use woothee::parser::Parser;

pub(crate) struct WootheeEngine {
    parser: Parser,
}

impl WootheeEngine {
    pub(crate) fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    pub(crate) fn parse(&self, ua: &str) -> UserAgentInfo {
        let Some(result) = self.parser.parse(ua) else {
            return UserAgentInfo {
                device_type: DeviceType::Unknown,
                is_bot: false,
            };
        };

        let device_type = match result.category {
            "pc" => DeviceType::Desktop,
            "smartphone" => DeviceType::Mobile,
            "tablet" => DeviceType::Tablet,
            "crawler" => DeviceType::Bot,
            _ => DeviceType::Unknown,
        };

        let is_bot = result.category == "crawler";

        UserAgentInfo {
            device_type,
            is_bot,
        }
    }
}
