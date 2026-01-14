use crate::conf::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BindInterfaceSpec {
    /// 127.0.0.1 / ::1
    #[default]
    Loopback,
    /// 0.0.0.0 / ::
    All,
    /// Custom IP address defined by an operator.
    Ip(std::net::IpAddr),
}

impl BindInterfaceSpec {
    pub fn as_ip(&self) -> IpAddr {
        match self {
            BindInterfaceSpec::Loopback => IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            BindInterfaceSpec::All => IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            BindInterfaceSpec::Ip(ip) => ip.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BindInterfaceInput {
    Keyword(String),
}

impl Default for BindInterfaceInput {
    fn default() -> Self {
        Self::Keyword("".to_string())
    }
}

impl TryFrom<BindInterfaceInput> for BindInterfaceSpec {
    type Error = ConfigError;

    fn try_from(input: BindInterfaceInput) -> Result<Self, Self::Error> {
        match input {
            BindInterfaceInput::Keyword(s) => match s.as_str() {
                "loopback" => Ok(BindInterfaceSpec::Loopback),
                "all" => Ok(BindInterfaceSpec::All),
                _ => {
                    let ip = IpAddr::from_str(&s)
                        .map_err(|_| ConfigError::InvalidBindIpString(s.clone()))?;
                    Ok(BindInterfaceSpec::Ip(ip))
                }
            },
        }
    }
}

impl fmt::Display for BindInterfaceInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindInterfaceInput::Keyword(s) => write!(f, "{s}"),
        }
    }
}
