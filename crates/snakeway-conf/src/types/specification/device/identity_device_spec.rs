use crate::types::HclInt;
use crate::validation::validate_trusted_proxies;
use crate::validation::validator::{validate_geoip_db_file, validate_ua_parser_regexes_file};
use confval::prelude::{KeywordSet, Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;
use std::path::PathBuf;

range_constraint!(MAX_X_FORWARDED_FOR_LENGTH, i64, min: 1, max: 2048);
range_constraint!(MAX_USER_AGENT_LENGTH, i64, min: 1, max: 4096);

pub const UA_ENGINES: [&str; 2] = ["uaparser", "woothee"];

#[derive(Clone, Debug, Serialize, confval::Spec)]
pub struct IdentityDeviceSpec {
    pub enable: Located<bool>,

    /// CIDR strings
    pub trusted_proxies: Vec<Located<String>>,
    #[confval(default = 1024)]
    pub max_x_forwarded_for_length: Located<HclInt>,

    pub enable_geoip: Located<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_city_db: Option<Located<PathBuf>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_isp_db: Option<Located<PathBuf>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geoip_connection_type_db: Option<Located<PathBuf>>,

    pub enable_user_agent: Located<bool>,
    pub ua_engine: Located<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ua_parser_regexes: Option<Located<PathBuf>>,
    #[confval(default = 2048)]
    pub max_user_agent_length: Located<HclInt>,
}

impl Default for IdentityDeviceSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            trusted_proxies: vec![],
            max_x_forwarded_for_length: Located::detached(1024),
            enable_geoip: Located::detached(false),
            geoip_city_db: None,
            geoip_isp_db: None,
            geoip_connection_type_db: None,
            enable_user_agent: Located::detached(false),
            ua_engine: Located::detached("woothee".to_string()),
            ua_parser_regexes: None,
            max_user_agent_length: Located::detached(2048),
        }
    }
}

impl Validate for IdentityDeviceSpec {
    fn validate(&self, report: &mut Report) {
        if !self.enable.value {
            return;
        }

        validate_trusted_proxies(&self.trusted_proxies, report);

        MAX_X_FORWARDED_FOR_LENGTH.check_located(
            &self.max_x_forwarded_for_length,
            "max_x_forwarded_for_length",
            report,
        );

        if self.enable_user_agent.value {
            MAX_USER_AGENT_LENGTH.check_located(
                &self.max_user_agent_length,
                "max_user_agent_length",
                report,
            );
        }

        KeywordSet::new(&UA_ENGINES).check_located(&self.ua_engine, "ua_engine", report);

        if self.enable_geoip.value {
            if let Some(path) = self.geoip_city_db.as_ref() {
                validate_geoip_db_file(path, report);
            }

            if let Some(path) = self.geoip_isp_db.as_ref() {
                validate_geoip_db_file(path, report);
            }

            if let Some(path) = self.geoip_connection_type_db.as_ref() {
                validate_geoip_db_file(path, report);
            }
        }

        if let Some(path) = self.ua_parser_regexes.as_ref() {
            validate_ua_parser_regexes_file(path, report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_xff_length_below_range() {
        // Arrange
        let mut report = Report::new();
        let spec = IdentityDeviceSpec {
            enable: Located::detached(true),
            max_x_forwarded_for_length: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("max_x_forwarded_for_length"))
        );
    }

    #[test]
    fn max_xff_length_above_range() {
        // Arrange
        let mut report = Report::new();
        let spec = IdentityDeviceSpec {
            enable: Located::detached(true),
            max_x_forwarded_for_length: Located::detached(2049),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("max_x_forwarded_for_length"))
        );
    }

    #[test]
    fn unknown_ua_engine_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = IdentityDeviceSpec {
            enable: Located::detached(true),
            ua_engine: Located::detached("regex9000".to_string()),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown ua_engine: regex9000")
        );
    }

    #[test]
    fn valid_identity_device() {
        // Arrange
        let mut report = Report::new();
        let spec = IdentityDeviceSpec {
            enable: Located::detached(true),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn disabled_device_skips_validation() {
        // Arrange
        let mut report = Report::new();
        let spec = IdentityDeviceSpec {
            enable: Located::detached(false),
            max_x_forwarded_for_length: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }
}
