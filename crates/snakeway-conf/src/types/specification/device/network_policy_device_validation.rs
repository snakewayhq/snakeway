use crate::types::{NetworkPolicyDeviceSpec, Origin};
use crate::validation::{ValidateSpec, ValidationReport};
use ipnet::IpNet;

impl ValidateSpec for NetworkPolicyDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        for cidr in &self.cidr_allow {
            if cidr.parse::<IpNet>().is_err() {
                report.invalid_network_policy_cidr(cidr, origin);
            }
        }
    }
}
