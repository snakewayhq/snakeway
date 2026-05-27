use crate::types::{IdentityDeviceSpec, UaEngineSpec};
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(o2o, Default, Clone, Debug, Deserialize, Serialize)]
#[from_owned(IdentityDeviceSpec)]
#[serde(deny_unknown_fields)]
pub struct IdentityDeviceConfig {
    pub enable: bool,

    /// CIDR strings
    pub trusted_proxies: Vec<String>,

    pub max_x_forwarded_for_length: usize,

    pub enable_geoip: bool,

    pub geoip_city_db: Option<PathBuf>,
    pub geoip_isp_db: Option<PathBuf>,
    pub geoip_connection_type_db: Option<PathBuf>,

    pub enable_user_agent: bool,

    #[map(~.into())]
    pub ua_engine: UaEngineKind,

    pub ua_parser_regexes: Option<PathBuf>,

    pub max_user_agent_length: usize,
}

#[derive(o2o, Default, Debug, Deserialize, Serialize, Clone, Copy)]
#[from_owned(UaEngineSpec)]
#[serde(rename_all = "lowercase")]
pub enum UaEngineKind {
    UaParser,
    #[default]
    Woothee,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_fields() {
        // Arrange
        let spec = IdentityDeviceSpec {
            enable: true,
            trusted_proxies: vec!["10.0.0.0/8".to_string()],
            max_x_forwarded_for_length: 512,
            enable_geoip: true,
            geoip_city_db: Some(PathBuf::from("/data/city.mmdb")),
            geoip_isp_db: None,
            geoip_connection_type_db: None,
            enable_user_agent: true,
            ua_engine: UaEngineSpec::UaParser,
            ua_parser_regexes: Some(PathBuf::from("/data/regexes.yaml")),
            max_user_agent_length: 1024,
            ..Default::default()
        };

        // Act
        let config: IdentityDeviceConfig = spec.into();

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
}
