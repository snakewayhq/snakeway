use crate::types::{IdentityDeviceSpec, Origin};
use crate::validation::validator::{validate_geoip_db_file, validate_ua_parser_regexes_file};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, validate_trusted_proxies,
};

const MAX_X_FORWARDED_FOR_LENGTH: RangeConstraint<usize> = RangeConstraint {
    min: 1,
    max: 2024,
    label: "max_x_forwarded_for_length",
    units: None,
};

const MAX_USER_AGENT_LENGTH: RangeConstraint<usize> = RangeConstraint {
    min: 1,
    max: 4096,
    label: "max_user_agent_length",
    units: None,
};

impl ValidateSpec for IdentityDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_trusted_proxies(&self.trusted_proxies, report, origin);

        MAX_X_FORWARDED_FOR_LENGTH.validate(self.max_x_forwarded_for_length, report, origin);

        if self.enable_user_agent {
            MAX_USER_AGENT_LENGTH.validate(self.max_user_agent_length, report, origin);
        }

        if self.enable_geoip {
            if let Some(path) = self.geoip_city_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_isp_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }

            if let Some(path) = self.geoip_connection_type_db.as_ref() {
                validate_geoip_db_file(path, report, origin);
            }
        }

        if let Some(path) = self.ua_parser_regexes.as_ref() {
            validate_ua_parser_regexes_file(path, report, origin);
        }
    }
}
