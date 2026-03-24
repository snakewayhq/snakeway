use crate::types::{
    ConnectionRateLimitingFilterSpec, NetworkConnectionFilterSpec, Origin, RedirectSpec,
    StaticRouteSpec, TlsTerminationSpec,
};
use crate::validation::report::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
use crate::validation::validator::{
    CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND,
    CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS, REDIRECT_RESPONSE_CODE, is_valid_port,
    validate_cert_key_pair, validate_range,
};

impl ValidateSpec for NetworkConnectionFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !self.ip_family.ipv4 && !self.ip_family.ipv6 {
            report.connection_filter_requires_at_least_one_ip_family(origin);
        }

        for cidr in &self.cidr.allow {
            if cidr.parse::<ipnet::IpNet>().is_err() {
                report.invalid_cidr_in_connection_filter_allow_list(cidr, origin);
            }
        }

        for cidr in &self.cidr.deny {
            if cidr.parse::<ipnet::IpNet>().is_err() {
                report.invalid_cidr_in_connection_filter_deny_list(cidr, origin);
            }
        }
    }
}

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.window_seconds,
            &CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS,
            report,
            origin,
        );

        validate_range(
            self.max_connections_per_second,
            &CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND,
            report,
            origin,
        );
    }
}

impl ValidateSpec for TlsTerminationSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        match self {
            TlsTerminationSpec::Manual { cert, key } => {
                if let Err(e) = validate_cert_key_pair(cert, key) {
                    report.ingress_tls_manual_cert_pair_invalid(&e, origin);
                }
            }
            TlsTerminationSpec::Acme { domains, .. } => {
                if domains.is_empty() {
                    report.acme_tls_requires_domains(origin);
                }
            }
        }
    }
}

impl ValidateSpec for RedirectSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        validate_range(self.status, &REDIRECT_RESPONSE_CODE, report, origin);
    }
}

impl ValidateSpec for StaticRouteSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if !self.file_dir.exists() {
            report.invalid_static_dir(&self.file_dir, origin);
        }
        if self.file_dir.is_relative() {
            report.invalid_static_dir_must_be_absolute(&self.file_dir, origin);
        }
    }
}
