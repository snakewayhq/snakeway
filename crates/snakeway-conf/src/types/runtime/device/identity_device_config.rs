use crate::types::IdentityDeviceSpec;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceConfig {
    pub enable: bool,

    pub trusted_proxies: Vec<String>,

    pub max_x_forwarded_for_length: usize,

    pub enable_geoip: bool,

    pub geoip_city_db: Option<PathBuf>,
    pub geoip_isp_db: Option<PathBuf>,
    pub geoip_connection_type_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    pub ua_engine: UaEngineKind,

    pub ua_parser_regexes: Option<PathBuf>,

    pub max_user_agent_length: usize,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    #[default]
    Woothee,
}

impl TryFrom<&str> for UaEngineKind {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, Self::Error> {
        match keyword {
            "uaparser" => Ok(UaEngineKind::UaParser),
            "woothee" => Ok(UaEngineKind::Woothee),
            other => Err(format!("unknown ua_engine: {other}")),
        }
    }
}

impl TryFrom<IdentityDeviceSpec> for IdentityDeviceConfig {
    type Error = String;

    fn try_from(spec: IdentityDeviceSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            enable: spec.enable.value,
            trusted_proxies: spec.trusted_proxies.into_iter().map(|p| p.value).collect(),
            max_x_forwarded_for_length: spec.max_x_forwarded_for_length.value as usize,
            enable_geoip: spec.enable_geoip.value,
            geoip_city_db: spec.geoip_city_db.map(|p| p.value),
            geoip_isp_db: spec.geoip_isp_db.map(|p| p.value),
            geoip_connection_type_db: spec.geoip_connection_type_db.map(|p| p.value),
            enable_user_agent: spec.enable_user_agent.value,
            ua_engine: spec.ua_engine.value.as_str().try_into()?,
            ua_parser_regexes: spec.ua_parser_regexes.map(|p| p.value),
            max_user_agent_length: spec.max_user_agent_length.value as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Located;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = IdentityDeviceSpec {
            enable: Located::detached(true),
            trusted_proxies: vec![Located::detached("10.0.0.0/8".to_string())],
            max_x_forwarded_for_length: Located::detached(512),
            enable_geoip: Located::detached(true),
            geoip_city_db: Some(Located::detached(PathBuf::from("/data/city.mmdb"))),
            geoip_isp_db: None,
            geoip_connection_type_db: None,
            enable_user_agent: Located::detached(true),
            ua_engine: Located::detached("uaparser".to_string()),
            ua_parser_regexes: Some(Located::detached(PathBuf::from("/data/regexes.yaml"))),
            max_user_agent_length: Located::detached(1024),
        };

        // Act
        let config = IdentityDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert!(config.enable);
        assert_eq!(config.trusted_proxies, vec!["10.0.0.0/8"]);
        assert_eq!(config.max_x_forwarded_for_length, 512);
        assert!(config.enable_geoip);
        assert_eq!(config.geoip_city_db, Some(PathBuf::from("/data/city.mmdb")));
        assert!(config.geoip_isp_db.is_none());
        assert!(config.enable_user_agent);
        assert!(matches!(config.ua_engine, UaEngineKind::UaParser));
        assert_eq!(
            config.ua_parser_regexes,
            Some(PathBuf::from("/data/regexes.yaml"))
        );
        assert_eq!(config.max_user_agent_length, 1024);
    }

    #[test]
    fn unknown_ua_engine_fails() {
        // Arrange
        let spec = IdentityDeviceSpec {
            ua_engine: Located::detached("psychic".to_string()),
            ..Default::default()
        };

        // Act
        let result = IdentityDeviceConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
    }
}
