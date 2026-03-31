use crate::types::{IdentityDeviceSpec, UaEngineSpec};

use super::{IdentityDeviceConfig, UaEngineKind};

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
            ua_parser_regexes: spec.ua_parser_regexes,
            max_user_agent_length: spec.max_user_agent_length,
        }
    }
}

impl From<UaEngineSpec> for UaEngineKind {
    fn from(ua_engine: UaEngineSpec) -> Self {
        match ua_engine {
            UaEngineSpec::UaParser => UaEngineKind::UaParser,
            UaEngineSpec::Woothee => UaEngineKind::Woothee,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
