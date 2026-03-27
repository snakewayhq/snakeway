use crate::types::{IdentityDeviceSpec, Origin};
use crate::validation::validator::{
    IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH, IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
    validate_range,
};
use crate::validation::validator::{validate_geoip_db_file, validate_ua_parser_regexes_file};
use crate::validation::{ValidateSpec, ValidationReport, validate_trusted_proxies};

impl ValidateSpec for IdentityDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_trusted_proxies(&self.trusted_proxies, report, origin);

        validate_range(
            self.max_x_forwarded_for_length,
            &IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH,
            report,
            origin,
        );

        if self.enable_user_agent {
            validate_range(
                self.max_user_agent_length,
                &IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH,
                report,
                origin,
            );
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
