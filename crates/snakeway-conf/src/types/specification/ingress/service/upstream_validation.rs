use crate::types::{EndpointSpec, EndpointTlsSpec, HostSpec, Origin, UpstreamSpec};
use crate::validation::validator::{is_valid_hostname, is_valid_port};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for UpstreamSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.weight == 0 || self.weight > 1_000 {
            report.invalid_upstream_weight(&self.weight, origin);
        }
    }
}

impl ValidateSpec for EndpointSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        match &self.host {
            HostSpec::Ip(ip) if ip.is_unspecified() || ip.is_multicast() => {
                report.invalid_upstream_ip(ip, origin);
            }
            HostSpec::Hostname(name) if !is_valid_hostname(name) => {
                report.invalid_upstream_hostname(name, origin);
            }
            _ => {}
        }

        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        if let Some(tls) = &self.tls {
            tls.validate(origin, report);
        }
    }
}

impl ValidateSpec for EndpointTlsSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.sni.trim().is_empty() {
            report.upstream_tls_sni_required(origin);
        }
    }
}
