use crate::types::{NetworkConnectionFilterSpec, Origin};
use crate::validation::{ValidateSpec, ValidationReport};

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
