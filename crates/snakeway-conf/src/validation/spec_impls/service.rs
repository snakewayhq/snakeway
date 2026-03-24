use crate::types::{
    CircuitBreakerSpec, EndpointSpec, EndpointTlsSpec, HostSpec, Origin, UpstreamSpec,
};
use crate::validation::report::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;
use crate::validation::validator::{
    CB_FAILURE_THRESHOLD, CB_HALF_OPEN_MAX_REQUESTS, CB_OPEN_DURATION_MS, CB_SUCCESS_THRESHOLD,
    is_valid_hostname, is_valid_port, validate_range,
};

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

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.failure_threshold,
            &CB_FAILURE_THRESHOLD,
            report,
            origin,
        );
        validate_range(
            self.open_duration_milliseconds,
            &CB_OPEN_DURATION_MS,
            report,
            origin,
        );
        validate_range(
            self.half_open_max_requests,
            &CB_HALF_OPEN_MAX_REQUESTS,
            report,
            origin,
        );
        validate_range(
            self.success_threshold,
            &CB_SUCCESS_THRESHOLD,
            report,
            origin,
        );
    }
}
